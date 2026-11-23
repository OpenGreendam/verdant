#!/usr/bin/env python3
"""Bind real-window CI observations to the same-run guarded Linux package.

The packaged release checker remains the authority for archive membership,
source-offer closure, executable identities, library pins and file modes.
This adapter selects its inputs and refuses incomplete or skipped OS runs.
"""

import argparse
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile

from probe import CASES, INITIAL_GAIN, sha256
from controls import CONTROL_CASES, validate_control_trace


PROGRAM_PATHS = {'verdant-gui': 'libexec/verdant/verdant-gui', 'verdant-worker': 'bin/verdant-worker'}
PROGRAM_ROLES = {'verdant-gui': 'gpui', 'verdant-worker': 'worker'}
SOURCE_FILES = ('frontend/Cargo.lock', 'frontend/proto-ui/compiler.pin.json',
                'tools/native-input/probe.py', 'tools/native-input/controls.py', 'tools/native-input/run-linux-isolated.py',
                'tools/native-input/ci.py', 'tools/native-input/x11_screenshot.py', 'tools/native-input/x11_keymap.py',
                'tools/a11y/run-isolated.py', 'tools/a11y/observe-atspi.py')


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def run_logged(command, evidence, label, **kwargs):
    result = subprocess.run([str(item) for item in command], text=True, capture_output=True,
                            timeout=180, **kwargs)
    (evidence / f'{label}.stdout.log').write_text(result.stdout)
    (evidence / f'{label}.stderr.log').write_text(result.stderr)
    write_json(evidence / f'{label}.command.json', {'command': list(map(str, command)),
                                                   'exitCode': result.returncode})
    if result.returncode:
        raise RuntimeError(f'{label} failed ({result.returncode}); see its retained stdout/stderr')
    return result.stdout


def observed_programs(manifest):
    """Bind observed roles after the release checker validates the full manifest."""
    if manifest.get('format') != 'verdant-native-package/3' or manifest.get('target') != 'linux-x86_64':
        raise RuntimeError('Unexpected Linux package manifest identity')
    entries = manifest.get('programs')
    if not isinstance(entries, list):
        raise RuntimeError('Missing packaged program identities')
    selected = {}
    for name, relative in PROGRAM_PATHS.items():
        matches = [entry for entry in entries if isinstance(entry, dict) and entry.get('name') == name]
        if len(matches) != 1 or matches[0].get('path') != relative \
                or matches[0].get('role') != PROGRAM_ROLES[name]:
            raise RuntimeError(f'Unexpected packaged identity for {name}')
        selected[name] = matches[0]
    return selected


def prepare(args):
    artifact = args.artifact_dir.resolve(strict=True)
    repo = args.repo.resolve(strict=True)
    work, evidence = args.work_dir.absolute(), args.evidence_dir.absolute()
    work.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    expected = args.expected_source_commit
    if not re.fullmatch(r'[0-9a-f]{40}', expected):
        raise RuntimeError('Expected source commit must be the full checked-out workflow commit')
    actual = run_logged(['git', '-C', repo, 'rev-parse', 'HEAD'], evidence, 'checkout-head').strip()
    if actual != expected:
        raise RuntimeError(f'Checkout {actual} does not match workflow commit {expected}')
    run_logged(['git', '-C', repo, 'diff', '--exit-code', 'HEAD', '--'], evidence, 'checkout-clean')

    entries = list(artifact.rglob('*'))
    if any(path.is_symlink() for path in entries):
        raise RuntimeError('Downloaded package artifact contains an unexpected symbolic link')
    files = [path for path in entries if path.is_file()]
    archives = [path for path in files if re.fullmatch(r'verdant-[A-Za-z0-9._+\-]+-linux-x86_64\.tar\.gz', path.name)]
    if len(archives) != 1:
        raise RuntimeError('Expected exactly one same-run Linux package archive')
    archive = archives[0]
    package_name = archive.name.removesuffix('.tar.gz')
    checksum = archive.with_name(package_name + '.sha256')
    if set(files) != {archive, checksum}:
        raise RuntimeError('The downloaded artifact must contain only the package archive and its checksum')
    archive_sha = sha256(archive)
    if checksum.read_text() != f'{archive_sha}  {archive.name}\n':
        raise RuntimeError('Linux package checksum does not match its archive')

    # Bootstrap only the fixed checker member, without extracting arbitrary
    # archive paths. Trust comes from download-artifact's same-workflow-run
    # producer job; the checksum is integrity evidence, not a signature.
    checker = work / 'verdant-release'
    with tarfile.open(archive, 'r:gz') as stream:
        members = [entry for entry in stream.getmembers()
                   if entry.name == f'{package_name}/bin/verdant-release']
        if len(members) != 1 or not members[0].isfile() or not 0 < members[0].size <= 256 * 1024 ** 2:
            raise RuntimeError('Package must contain exactly one bounded regular release checker')
        with checker.open('xb') as output, stream.extractfile(members[0]) as source:
            shutil.copyfileobj(source, output)
    checker.chmod(0o700)

    selection_path = evidence / 'source-selection.json'
    selected = json.loads(run_logged([checker, 'select-source', '--repo', repo,
                                      '--output', selection_path], evidence, 'source-selection'))
    selection_sha = selected.get('sha256')
    if selected.get('source_commit') != expected or selection_sha != sha256(selection_path):
        raise RuntimeError('Fresh source selection does not bind the expected checkout')
    check_options = ['--require-package', '--expected-source-commit', expected,
                     '--expected-source-selection-sha256', selection_sha]
    run_logged([checker, 'check', '--check', artifact, *check_options,
                '--write', evidence / 'archive-check.json'], evidence, 'archive-check')

    extracted = work / 'extracted'
    extracted.mkdir()
    # The existing release checker has already rejected links, special files,
    # escaping paths, duplicates, unexpected modes, members and bytes.
    run_logged(['tar', '--extract', '--gzip', '--file', archive, '--directory', extracted,
                '--no-same-owner', '--same-permissions'], evidence, 'extract-package')
    package = extracted / package_name
    run_logged([checker, 'check', '--check', package, *check_options,
                '--write', evidence / 'extracted-check.json'], evidence, 'extracted-check')
    manifest_path = package / 'RELEASE-ASSETS.json'
    manifest = json.loads(manifest_path.read_text())
    selected_programs = observed_programs(manifest)
    programs = {}
    for name, entry in selected_programs.items():
        path = package / entry['path']
        digest = sha256(path)
        if digest != entry['sha256']:
            raise RuntimeError(f'Extracted {name} differs from its verified manifest')
        programs[name] = {'path': str(path), 'sha256': digest, 'role': entry['role']}
        libraries = run_logged(['ldd', path], evidence, f'{name}-shared-libraries')
        if 'not found' in libraries:
            raise RuntimeError(f'Installed OS dependencies do not satisfy {name}; see its ldd evidence')
    shutil.copyfile(manifest_path, evidence / 'RELEASE-ASSETS.json')
    shutil.copyfile(checksum, evidence / checksum.name)
    pin = json.loads((repo / 'frontend/proto-ui/compiler.pin.json').read_text())
    identity = {'format': 'verdant.native-input-package.v1', 'sourceCommit': expected,
                'sourceSelectionSha256': selection_sha, 'compilerRevision': pin['upstream']['revision'],
                'archive': {'path': str(archive), 'sha256': archive_sha},
                'packageRoot': str(package), 'manifestSha256': sha256(manifest_path),
                'programs': programs, 'sourceFiles': {name: sha256(repo / name) for name in SOURCE_FILES},
                'trust': 'Same workflow-run linux artifact; existing release checker verified archive and extracted bytes against fresh checkout source selection',
                'environment': {'VERDANT_DATA_DIR': str(package / 'share/verdant'),
                                'VERDANT_PLUGIN_HOST': str(package / 'libexec/verdant/verdant-plughost'),
                                'VERDANT_WORKER_BINARY': programs['verdant-worker']['path'],
                                'VERDANT_ONNXRUNTIME_LIBRARY': str(package / 'lib/libonnxruntime.so.1.23.2')}}
    write_json(evidence / 'package.json', identity)
    print(json.dumps({'packageIdentity': str(evidence / 'package.json'), 'sourceCommit': expected,
                      'programs': programs}, ensure_ascii=False))
    return 0


def verify_identity(identity, repo=None):
    if identity.get('format') != 'verdant.native-input-package.v1':
        raise RuntimeError('Missing verified package identity')
    if sha256(Path(identity['packageRoot']) / 'RELEASE-ASSETS.json') != identity['manifestSha256']:
        raise RuntimeError('Package manifest changed after validation')
    for name in PROGRAM_PATHS:
        program = identity['programs'][name]
        if sha256(Path(program['path'])) != program['sha256']:
            raise RuntimeError(f'Packaged {name} changed after validation')
    if repo is not None:
        for name, digest in identity['sourceFiles'].items():
            if sha256(repo / name) != digest:
                raise RuntimeError(f'Observation source changed after validation: {name}')


def run_backend(args):
    identity = json.loads(args.package_info.read_text())
    repo = args.repo.resolve(strict=True)
    verify_identity(identity, repo)
    env = dict(os.environ, **identity['environment'])
    command = [sys.executable, str(repo / 'tools/native-input/run-linux-isolated.py'),
               '--deps-root', str(args.deps_root), '--repo', str(repo),
               '--binary', identity['programs']['verdant-gui']['path'],
               '--worker-binary', identity['programs']['verdant-worker']['path'],
               '--backend', args.backend, '--suite', args.suite,
               '--run-dir', str(args.run_dir), '--python', sys.executable]
    # No diagnostic options are accepted by this CI entry point.
    result = subprocess.run(command, env=env)
    verify_identity(identity, repo)
    return result.returncode


def evidence_file(root, relative):
    path = PurePosixPath(relative)
    if path.is_absolute() or '..' in path.parts:
        raise RuntimeError(f'Evidence path escapes its owned run: {relative!r}')
    actual = root / path
    if actual.is_symlink() or not actual.is_file():
        raise RuntimeError(f'Missing regular evidence file: {relative}')
    return actual


def validate_run(identity, backend, run):
    harness = json.loads((run / 'harness.json').read_text())
    evidence = run / 'evidence'
    report = json.loads((evidence / 'result.json').read_text())
    if harness.get('suite', 'original') != 'original' or report.get('suite', 'original') != 'original':
        raise RuntimeError(f'{backend}: original knob trace cannot be replaced by another suite')
    if harness.get('format') != 'verdant.isolated-native-run.v1' or harness.get('ok') is not True \
            or harness.get('exitCode') != 0 or harness.get('probeOptions') != []:
        raise RuntimeError(f'{backend}: isolated harness failed or used diagnostic filters')
    if report.get('format') != 'verdant.native-input-observation.v1' or report.get('ok') is not True \
            or report.get('skipSetValue') is not False or report.get('cleanupErrors'):
        raise RuntimeError(f'{backend}: probe failed, skipped Value writes or could not clean up')
    if harness.get('backend') != backend or report.get('backend') != backend:
        raise RuntimeError(f'{backend}: observation backend mismatch')
    if len(CASES) != 11 or report.get('cases') != [{'name': name, 'status': 'passed'} for name in CASES]:
        raise RuntimeError(f'{backend}: every one of the 11 expected cases must pass exactly once')
    for name, report_key, harness_key in [('verdant-gui', 'binary', 'binarySha256'),
                                          ('verdant-worker', 'worker', 'workerSha256')]:
        digest = identity['programs'][name]['sha256']
        if report.get(report_key, {}).get('sha256') != digest or harness.get(harness_key) != digest:
            raise RuntimeError(f'{backend}: {name} observation does not match the verified package')
    summaries = []
    for step in report.get('steps', []):
        detail_file = evidence_file(evidence, step['evidence'])
        detail = json.loads(detail_file.read_text())
        screenshot = evidence_file(evidence, step['screenshot'])
        document = evidence_file(evidence, str(PurePosixPath(step['evidence']).with_suffix('.verp')))
        if detail.get('label') != step['label'] or detail.get('case') != step.get('case') \
                or detail.get('savedProject') != step['savedProject'] \
                or sha256(document) != step['savedProject']['sha256']:
            raise RuntimeError(f'{backend}: step snapshot or document evidence differs: {step["label"]}')
        if not screenshot.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'):
            raise RuntimeError(f'{backend}: missing PNG screenshot for {step["label"]}')
        summaries.append({'label': step['label'], 'case': step.get('case'),
                          'savedProject': step['savedProject'],
                          'observedNormalizedValues': step['observedNormalizedValues'],
                          'targetFocused': ['focused' in node.get('states', []) for node in detail.get('target', [])],
                          'pointerHeld': step['pointerHeld'], 'window': step['window'],
                          'windowReference': step['windowReference'],
                          'evidenceSha256': sha256(detail_file), 'screenshotSha256': sha256(screenshot)})
    required_labels = {'preflight', 'keyboard-single-undo', 'drag-single-undo', 'escape-canceled',
                       'platform-focus-handoff-canceled', 'window-exit-outside-release-canceled',
                       'toolbar-remounted', 'atspi-value-single-undo', 'atspi-takeover-late-up',
                       'atspi-takeover-single-undo', 'lease-reusable-single-undo',
                       'native-closed-while-held', 'native-close-canceled', 'native-reopened-single-undo',
                       'native-preset-menu-opened', 'native-preset-canceled'}
    if not required_labels <= {step['label'] for step in summaries}:
        raise RuntimeError(f'{backend}: required interaction evidence is incomplete')
    if {step['case'] for step in summaries} != {None, *CASES}:
        raise RuntimeError(f'{backend}: every case must retain its own observed step evidence')
    final = json.loads((evidence / 'project.verp').read_text())
    gain = final['tracks'][0]['instrument']['params']['gain']
    if final['revision'] != 12 or not math.isfinite(gain) or abs(gain - INITIAL_GAIN) > 1e-6 \
            or sha256(evidence / 'project.verp') != summaries[-1]['savedProject']['sha256']:
        raise RuntimeError(f'{backend}: final saved gain/revision differs from the complete 11-case trace')
    if backend == 'wayland':
        if harness.get('weston', {}).get('major') not in (13, 14):
            raise RuntimeError('Wayland: actual supported Weston version was not recorded')
        protocol = report.get('waylandProtocol', {})
        events = protocol.get('incomingEvents', {})
        contacts = protocol.get('contactEvidence', {})
        if not events.get('wl_pointer', {}).get('button') or not events.get('wl_keyboard', {}).get('key'):
            raise RuntimeError('Wayland: missing real pointer/keyboard protocol events')
        if protocol.get('expectedEscapeContacts') != [True, True, True, True] \
                or [event['pointerHeld'] for event in contacts.get('escapeKeyEvents', [])] != [True] * 4:
            raise RuntimeError('Wayland: both Escape pairs must preserve the actual held pointer')
        if not any(event['pointerHeld'] for event in contacts.get('outsideMotions', [])) \
                or not contacts.get('outsideReleases'):
            raise RuntimeError('Wayland: actual window-exit motion and outside release were not observed')
        for mapping in report.get('windowMappings', []):
            surface = mapping.get('mapping', {}).get('sceneGraphSurface')
            if not surface:
                raise RuntimeError('Wayland: every window must have a measured scene graph surface')
            evidence_file(evidence, surface['evidence'])
    return {'backend': backend, 'suite': 'original', 'cases': report['cases'], 'binary': report['binary'], 'worker': report['worker'],
            'elapsedSeconds': report['elapsedSeconds'], 'finalProject': summaries[-1]['savedProject'],
            'steps': summaries, 'windowMappings': report['windowMappings'], 'weston': harness.get('weston'),
            'waylandProtocol': report.get('waylandProtocol'), 'tolerances': report['tolerances'],
            'resultSha256': sha256(evidence / 'result.json'), 'harnessSha256': sha256(run / 'harness.json')}


def validate_control_run(identity, backend, run):
    harness = json.loads((run / 'harness.json').read_text())
    evidence = run / 'evidence'
    report = json.loads((evidence / 'result.json').read_text())
    if harness.get('format') != 'verdant.isolated-native-run.v1' or harness.get('ok') is not True \
            or harness.get('exitCode') != 0 or harness.get('probeOptions') != []:
        raise RuntimeError(f'{backend} controls: isolated harness failed or used diagnostic filters')
    if report.get('format') != 'verdant.native-input-observation.v1' or report.get('ok') is not True \
            or report.get('skipSetValue') is not False or report.get('cleanupErrors'):
        raise RuntimeError(f'{backend} controls: probe failed or cleanup/skip evidence is incomplete')
    if any(item.get('backend') != backend or item.get('suite') != 'controls' for item in (harness, report)):
        raise RuntimeError(f'{backend} controls: backend/suite mismatch')
    if report.get('cases') != [{'name': name, 'status': 'passed'} for name in CONTROL_CASES]:
        raise RuntimeError(f'{backend} controls: all 3 expected cases must pass exactly once')
    for name, report_key, harness_key in [('verdant-gui', 'binary', 'binarySha256'),
                                          ('verdant-worker', 'worker', 'workerSha256')]:
        digest = identity['programs'][name]['sha256']
        if report.get(report_key, {}).get('sha256') != digest or harness.get(harness_key) != digest:
            raise RuntimeError(f'{backend} controls: {name} observation does not match the verified package')
    observations, summaries = [], []
    for step in report.get('steps', []):
        detail_file = evidence_file(evidence, step['evidence'])
        detail = json.loads(detail_file.read_text())
        screenshot = evidence_file(evidence, step['screenshot'])
        project_file = evidence_file(evidence, str(PurePosixPath(step['evidence']).with_suffix('.verp')))
        project = json.loads(project_file.read_text())
        saved = step['savedProject']
        if any(detail.get(key) != step.get(key) for key in ('label', 'case', 'savedProject', 'control', 'pointerHeld', 'window', 'windowReference')) \
                or sha256(project_file) != saved['sha256']:
            raise RuntimeError(f'{backend} controls: raw step/document evidence differs: {step["label"]}')
        track = project['tracks'][0]
        if saved['revision'] != project['revision'] or saved['trackName'] != track['name'] \
                or saved['panPermille'] != track.get('pan_permille', 0) \
                or saved['gainDb'] != track['instrument']['params']['gain']:
            raise RuntimeError(f'{backend} controls: summary does not describe the retained project')
        if not screenshot.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'):
            raise RuntimeError(f'{backend} controls: missing PNG observation')
        observations.append((detail, project))
        summaries.append({'label': step['label'], 'case': step.get('case'), 'savedProject': saved,
                          'observedNormalizedValues': step['observedNormalizedValues'],
                          'targetFocused': ['focused' in node.get('states', []) for node in detail.get('target', [])],
                          'control': detail.get('control'), 'pointerHeld': step['pointerHeld'],
                          'window': step['window'], 'windowReference': step['windowReference'],
                          'evidenceSha256': sha256(detail_file), 'screenshotSha256': sha256(screenshot)})
    validate_control_trace(observations, json.loads((evidence / 'project.verp').read_text()))
    if sha256(evidence / 'project.verp') != summaries[-1]['savedProject']['sha256']:
        raise RuntimeError(f'{backend} controls: final project differs from the last retained snapshot')
    if backend == 'wayland':
        if harness.get('weston', {}).get('major') not in (13, 14):
            raise RuntimeError('Wayland controls: supported Weston version was not recorded')
        protocol = report.get('waylandProtocol', {})
        events = protocol.get('incomingEvents', {})
        pointer = events.get('wl_pointer', {})
        if not pointer.get('button') or not events.get('wl_keyboard', {}).get('key') \
                or not any(pointer.get(key) for key in ('axis', 'axis_discrete', 'axis_value120')):
            raise RuntimeError('Wayland controls: missing real pointer/keyboard/wheel protocol events')
        expected = [False, False, False, False, True, True]
        if protocol.get('expectedEscapeContacts') != expected \
                or [event['pointerHeld'] for event in protocol.get('contactEvidence', {}).get('escapeKeyEvents', [])] != expected:
            raise RuntimeError('Wayland controls: cancellation did not preserve the actual held mixer pointer')
        for mapping in report.get('windowMappings', []):
            surface = mapping.get('mapping', {}).get('sceneGraphSurface')
            if not surface:
                raise RuntimeError('Wayland controls: every window needs measured compositor geometry')
            evidence_file(evidence, surface['evidence'])
    return {'backend': backend, 'suite': 'controls', 'cases': report['cases'],
            'binary': report['binary'], 'worker': report['worker'], 'elapsedSeconds': report['elapsedSeconds'],
            'finalProject': summaries[-1]['savedProject'], 'steps': summaries, 'windowMappings': report['windowMappings'],
            'weston': harness.get('weston'), 'waylandProtocol': report.get('waylandProtocol'), 'tolerances': report['tolerances'],
            'resultSha256': sha256(evidence / 'result.json'), 'harnessSha256': sha256(run / 'harness.json')}


def compact_summary(report):
    """Only verified observations; this is also printed for job-log retrieval."""
    identity = report.get('package', {})
    summary = {'format': 'verdant.native-input-ci-summary.v1', 'ok': report['ok'],
               'expectedCasesPerBackend': 11, 'expectedControlCasesPerBackend': 3,
               'passedCases': sum(len(run['cases']) for run in report['runs']),
               'originalPassedCases': sum(len(run['cases']) for run in report['runs'] if run['suite'] == 'original'),
               'controlPassedCases': sum(len(run['cases']) for run in report['runs'] if run['suite'] == 'controls'),
               'sourceCommit': identity.get('sourceCommit'), 'sourceSelectionSha256': identity.get('sourceSelectionSha256'),
               'compilerRevision': identity.get('compilerRevision'), 'manifestSha256': identity.get('manifestSha256'),
               'archiveSha256': identity.get('archive', {}).get('sha256'),
               'sourceFiles': identity.get('sourceFiles'), 'errors': report['errors'], 'runs': []}
    for run in report['runs']:
        cases = []
        for case in run['cases']:
            observations = []
            for step in run['steps']:
                if step['case'] != case['name']:
                    continue
                saved = step['savedProject']
                observations.append({'label': step['label'], 'gainDb': saved['gainDb'],
                                     'trackName': saved.get('trackName'), 'panPermille': saved.get('panPermille'),
                                     'revision': saved['revision'], 'projectSha256': saved['sha256'],
                                     'normalizedValues': step['observedNormalizedValues'], 'focused': step['targetFocused'],
                                     'pointerHeld': step['pointerHeld'], 'window': step['window']})
            cases.append({**case, 'observations': observations})
        summary['runs'].append({'backend': run['backend'], 'suite': run['suite'], 'cases': cases,
                               'binarySha256': run['binary']['sha256'], 'workerSha256': run['worker']['sha256'],
                               'elapsedSeconds': run['elapsedSeconds'], 'finalProject': run['finalProject'],
                               'westonVersion': (run.get('weston') or {}).get('version'),
                               'waylandProtocol': run.get('waylandProtocol'), 'tolerances': run['tolerances'],
                               'resultSha256': run['resultSha256'], 'harnessSha256': run['harnessSha256']})
    return summary


def retain_run(run, destination):
    """Copy all owned ordinary evidence files; never follow sockets or links."""
    skipped = []
    if not run.is_dir():
        return [{'path': str(run), 'reason': 'run directory was not created'}]
    destination.mkdir(parents=True, exist_ok=False)
    for directory, directories, files in os.walk(run, followlinks=False):
        current = Path(directory)
        for name in list(directories):
            if (current / name).is_symlink():
                directories.remove(name)
                skipped.append({'path': str((current / name).relative_to(run)), 'reason': 'symlink'})
        for name in files:
            source = current / name
            relative = source.relative_to(run)
            if not stat.S_ISREG(source.lstat().st_mode):
                skipped.append({'path': str(relative), 'reason': 'non-regular runtime endpoint'})
                continue
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
    return skipped


def check(args):
    output = args.output.absolute()
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {'format': 'verdant.native-input-ci.v1', 'ok': False, 'expectedCasesPerBackend': 11, 'expectedControlCasesPerBackend': 3,
              'runs': [], 'errors': [], 'retention': {}}
    identity = None
    try:
        identity = json.loads(args.package_info.read_text())
        verify_identity(identity)
        report['package'] = identity
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        report['errors'].append(f'Package identity: {error}')
    runs = [('x11', 'original', args.x11_run), ('wayland', 'original', args.wayland_run),
            ('x11', 'controls', args.controls_x11_run), ('wayland', 'controls', args.controls_wayland_run)]
    for backend, suite, run in runs:
        name = backend if suite == 'original' else f'{suite}-{backend}'
        try:
            if identity is None:
                raise RuntimeError('No verified package identity')
            validate = validate_run if suite == 'original' else validate_control_run
            report['runs'].append(validate(identity, backend, run))
        except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
            report['errors'].append(f'{name}: {error}')
        finally:
            try:
                report['retention'][name] = retain_run(run, output.parent / name)
            except OSError as error:
                report['errors'].append(f'{name} evidence retention: {error}')
    report['ok'] = not report['errors'] and len(report['runs']) == 4
    write_json(output, report)
    summary = compact_summary(report)
    write_json(output.with_name('summary.json'), summary)
    print('BEGIN_VERDANT_NATIVE_INPUT_JSON', flush=True)
    print(json.dumps(summary, ensure_ascii=False, separators=(',', ':')), flush=True)
    print('END_VERDANT_NATIVE_INPUT_JSON', flush=True)
    print(json.dumps({'ok': report['ok'], 'passedCases': sum(len(run['cases']) for run in report['runs']),
                      'errors': report['errors'], 'evidence': str(output)}, ensure_ascii=False))
    return 0 if report['ok'] else 1


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    prepare_parser = commands.add_parser('prepare', help='Verify and extract the same-run guarded Linux artifact')
    for option in ('artifact-dir', 'repo', 'work-dir', 'evidence-dir'):
        prepare_parser.add_argument('--' + option, type=Path, required=True)
    prepare_parser.add_argument('--expected-source-commit', required=True)
    run_parser = commands.add_parser('run', help='Run one complete backend using verified packaged programs')
    for option in ('package-info', 'repo', 'run-dir'):
        run_parser.add_argument('--' + option, type=Path, required=True)
    run_parser.add_argument('--deps-root', type=Path, default=Path('/'))
    run_parser.add_argument('--backend', choices=('x11', 'wayland'), required=True)
    run_parser.add_argument('--suite', choices=('original', 'controls'), default='original')
    check_parser = commands.add_parser('check', help='Require the original 22 and 6 control passes, retaining all raw evidence')
    for option in ('package-info', 'x11-run', 'wayland-run', 'controls-x11-run', 'controls-wayland-run', 'output'):
        check_parser.add_argument('--' + option, type=Path, required=True)
    return parser.parse_args()


if __name__ == '__main__':
    try:
        args = arguments()
        sys.exit({'prepare': prepare, 'run': run_backend, 'check': check}[args.command](args))
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(f'Native input CI rejected: {error}', file=sys.stderr)
        sys.exit(1)
