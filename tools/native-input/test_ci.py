#!/usr/bin/env python3
"""Offline CI refusal checks, not native-window observations."""

import copy
import json
from pathlib import Path
import runpy
import tempfile
import unittest

from ci import PROGRAM_PATHS, observed_programs, validate_run, verify_identity, write_json
from probe import CASES, sha256


class CiRefusals(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.run = self.root / 'run'
        self.evidence = self.run / 'evidence'
        self.evidence.mkdir(parents=True)
        package = self.root / 'package'
        package.mkdir()
        (package / 'RELEASE-ASSETS.json').write_text('synthetic test manifest\n')
        self.identity = {'format': 'verdant.native-input-package.v1', 'packageRoot': str(package),
                         'manifestSha256': sha256(package / 'RELEASE-ASSETS.json'), 'programs': {}}
        for name, relative in PROGRAM_PATHS.items():
            path = package / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('synthetic test bytes: ' + name)
            self.identity['programs'][name] = {'path': str(path), 'sha256': sha256(path)}
        self.harness = {'format': 'verdant.isolated-native-run.v1', 'backend': 'x11', 'ok': True,
                        'exitCode': 0, 'probeOptions': [],
                        'binarySha256': self.identity['programs']['verdant-gui']['sha256'],
                        'workerSha256': self.identity['programs']['verdant-worker']['sha256']}
        self.report = {'format': 'verdant.native-input-observation.v1', 'backend': 'x11', 'ok': True,
                       'skipSetValue': False, 'cases': [{'name': name, 'status': 'passed'} for name in CASES],
                       'binary': self.identity['programs']['verdant-gui'],
                       'worker': self.identity['programs']['verdant-worker']}
        self.save()

    def save(self):
        write_json(self.run / 'harness.json', self.harness)
        write_json(self.evidence / 'result.json', self.report)

    def test_package_roles_select_gui_and_refuse_cli_or_launcher_aliases(self):
        manifest = {'format': 'verdant-native-package/3', 'target': 'linux-x86_64',
                    'programs': [
                        {'name': 'verdant', 'role': 'cli', 'path': 'bin/verdant', 'sha256': '1' * 64},
                        {'name': 'verdant-gui', 'role': 'gpui', 'path': 'libexec/verdant/verdant-gui', 'sha256': '2' * 64},
                        {'name': 'verdant-launcher', 'role': 'launcher', 'path': 'verdant-launcher', 'sha256': '3' * 64},
                        {'name': 'verdant-worker', 'role': 'worker', 'path': 'bin/verdant-worker', 'sha256': '4' * 64},
                    ]}
        selected = observed_programs(manifest)
        self.assertEqual(set(selected), {'verdant-gui', 'verdant-worker'})
        self.assertEqual(selected['verdant-gui']['sha256'], '2' * 64)
        self.assertEqual(selected['verdant-worker']['sha256'], '4' * 64)
        for field, value in (('name', 'verdant'), ('name', 'verdant-launcher'),
                             ('role', 'cli'), ('role', 'launcher'),
                             ('path', 'bin/verdant'), ('path', 'verdant-launcher'),
                             ('path', 'libexec/verdant/verdant')):
            with self.subTest(field=field, value=value):
                changed = copy.deepcopy(manifest)
                changed['programs'][1][field] = value
                with self.assertRaisesRegex(RuntimeError, 'packaged identity'):
                    observed_programs(changed)
        for programs in (manifest['programs'][:1] + manifest['programs'][2:],
                         manifest['programs'] + [manifest['programs'][1]]):
            with self.subTest(programs=programs):
                with self.assertRaisesRegex(RuntimeError, 'packaged identity'):
                    observed_programs({**manifest, 'programs': programs})
        with self.assertRaisesRegex(RuntimeError, 'manifest identity'):
            observed_programs({**manifest, 'format': 'verdant-native-package/2'})

    def test_missing_result_never_becomes_a_skip(self):
        (self.evidence / 'result.json').unlink()
        with self.assertRaises(FileNotFoundError):
            validate_run(self.identity, 'x11', self.run)

    def test_skipped_missing_or_duplicate_cases_are_rejected(self):
        original = copy.deepcopy(self.report['cases'])
        for cases in (original[:-1], original + [original[-1]],
                      [{**case, 'status': 'skipped'} if case['name'] == 'set-value' else case for case in original]):
            with self.subTest(cases=cases):
                self.report['cases'] = cases
                self.save()
                with self.assertRaisesRegex(RuntimeError, '11 expected cases'):
                    validate_run(self.identity, 'x11', self.run)

    def test_filtered_run_and_wrong_package_bytes_are_rejected(self):
        self.harness['probeOptions'] = ['--only', 'keyboard']
        self.save()
        with self.assertRaisesRegex(RuntimeError, 'diagnostic filters'):
            validate_run(self.identity, 'x11', self.run)
        self.harness['probeOptions'] = []
        self.report['worker'] = {'sha256': '0' * 64}
        self.save()
        with self.assertRaisesRegex(RuntimeError, 'verified package'):
            validate_run(self.identity, 'x11', self.run)

    def test_manifest_and_executable_identity_is_rechecked(self):
        verify_identity(self.identity)
        path = Path(self.identity['programs']['verdant-worker']['path'])
        path.write_text('changed after validation')
        with self.assertRaisesRegex(RuntimeError, 'verdant-worker changed'):
            verify_identity(self.identity)
        self.identity['programs']['verdant-worker']['sha256'] = sha256(path)
        (Path(self.identity['packageRoot']) / 'RELEASE-ASSETS.json').write_text('changed manifest')
        with self.assertRaisesRegex(RuntimeError, 'manifest changed'):
            verify_identity(self.identity)

    def test_supported_weston_version_is_observed_not_assumed(self):
        check = runpy.run_path(str(Path(__file__).with_name('run-linux-isolated.py')))['weston_major']
        help_text = '--renderer=NAME\n  pixman\n --debug\n'
        self.assertEqual(check('weston 13.0.0', help_text), 13)
        self.assertEqual(check('weston 14.0.2', help_text), 14)
        with self.assertRaisesRegex(RuntimeError, 'Unsupported Weston'):
            check('weston 12.0.0', help_text)
        with self.assertRaisesRegex(RuntimeError, 'advertise'):
            check('weston 13.0.0', '--debug')

    def test_weston_output_id_comes_from_the_actual_owned_server_record(self):
        parse = runpy.run_path(str(Path(__file__).with_name('run-linux-isolated.py')))['weston_x11_output']
        line = '[13:01:13.066] x11 output 1600x1000, window id 2097157'
        output = parse('other startup log\n' + line + '\n')
        self.assertEqual(output, {'width': 1600, 'height': 1000, 'window': '2097157', 'logLine': line})
        self.assertIsNone(parse('x11 output size 1600x1000 without an observed ID'))
        with self.assertRaisesRegex(RuntimeError, 'invalid X11 output'):
            parse(line.replace('2097157', '0'))
        with self.assertRaisesRegex(RuntimeError, 'one observed X11 output'):
            parse(line + '\n' + line.replace('2097157', '2097158'))


from ci import validate_control_run
from controls import CONTROL_CASES
from test_probe import synthetic_control_observations


class ControlCiRefusals(unittest.TestCase):
    setUp = CiRefusals.setUp
    save = CiRefusals.save

    def setup_controls(self):
        self.harness['suite'] = self.report['suite'] = 'controls'
        self.report['cases'] = [{'name': name, 'status': 'passed'} for name in CONTROL_CASES]
        self.report.update({'steps': [], 'windowMappings': [], 'elapsedSeconds': 1.0, 'tolerances': {}})
        observations, final = synthetic_control_observations()
        for index, (detail, document) in enumerate(observations):
            stem = f'steps/{index:02}-{detail["label"]}'
            write_json(self.evidence / (stem + '.verp'), document)
            track = document['tracks'][0]
            saved = {'revision': document['revision'], 'trackName': track['name'],
                     'panPermille': track.get('pan_permille', 0), 'gainDb': track['instrument']['params']['gain'],
                     'sha256': sha256(self.evidence / (stem + '.verp'))}
            detail.update({'savedProject': saved, 'window': 'fixture', 'windowReference': [':1.2', '/frame'], 'target': []})
            write_json(self.evidence / (stem + '.json'), detail)
            (self.evidence / (stem + '.png')).write_bytes(b'\x89PNG\r\n\x1a\nsynthetic-evidence')
            self.report['steps'].append({key: detail[key] for key in
                ('label', 'case', 'control', 'pointerHeld', 'savedProject', 'window', 'windowReference')} |
                {'evidence': stem + '.json', 'screenshot': stem + '.png', 'observedNormalizedValues': []})
        write_json(self.evidence / 'project.verp', final)
        self.save()

    def test_original_and_controls_cannot_substitute_for_each_other(self):
        self.harness['suite'] = 'controls'
        self.save()
        with self.assertRaisesRegex(RuntimeError, 'cannot be replaced'):
            validate_run(self.identity, 'x11', self.run)
        with self.assertRaisesRegex(RuntimeError, 'suite mismatch'):
            validate_control_run(self.identity, 'x11', self.run)

    def test_all_three_control_cases_are_required_without_skips_or_duplicates(self):
        self.setup_controls()
        original = copy.deepcopy(self.report['cases'])
        for cases in (original[:-1], original + [original[-1]],
                      [{**case, 'status': 'skipped'} for case in original]):
            self.report['cases'] = cases
            self.save()
            with self.assertRaisesRegex(RuntimeError, '3 expected cases'):
                validate_control_run(self.identity, 'x11', self.run)

    def test_actual_project_bytes_and_summary_are_both_rechecked(self):
        self.setup_controls()
        self.assertEqual(len(validate_control_run(self.identity, 'x11', self.run)['cases']), 3)
        step = next(step for step in self.report['steps'] if step['label'] == 'mixer-committed')
        step['savedProject']['panPermille'] = 0
        detail = json.loads((self.evidence / step['evidence']).read_text())
        detail['savedProject'] = step['savedProject']
        write_json(self.evidence / step['evidence'], detail)
        self.save()
        with self.assertRaisesRegex(RuntimeError, 'retained project'):
            validate_control_run(self.identity, 'x11', self.run)



class DisplayFdProtocol(unittest.TestCase):
    def reader(self):
        return runpy.run_path(str(Path(__file__).with_name('run-linux-isolated.py')))['read_display_number']

    def test_separate_number_and_newline_writes_keep_the_reader_open(self):
        import os
        import threading
        from unittest.mock import patch
        reader = self.reader()
        read_fd, write_fd = os.pipe()
        observed_first = threading.Event()
        failures = []
        original_read = os.read
        def observed_read(fd, size):
            value = original_read(fd, size)
            if fd == read_fd and value == b'12':
                observed_first.set()
            return value
        def writer():
            try:
                os.write(write_fd, b'12')
                if not observed_first.wait(2):
                    raise RuntimeError('Reader did not consume the deliberately split first write')
                os.write(write_fd, b'\n')
            except Exception as error:
                failures.append(error)
            finally:
                os.close(write_fd)
        thread = threading.Thread(target=writer)
        thread.start()
        try:
            with patch('os.read', observed_read):
                self.assertEqual(reader(read_fd, timeout=2), '12')
        finally:
            os.close(read_fd)
            thread.join(3)
        self.assertFalse(thread.is_alive())
        self.assertEqual(failures, [])

    def test_truncated_invalid_and_unbounded_lines_are_rejected(self):
        import os
        reader = self.reader()
        for payload in (b'12', b'\n', b'-1\n', b'0\n99', b'0' * 65):
            with self.subTest(payload=payload):
                read_fd, write_fd = os.pipe()
                os.write(write_fd, payload)
                os.close(write_fd)
                try:
                    with self.assertRaises(RuntimeError):
                        reader(read_fd, timeout=0.2)
                finally:
                    os.close(read_fd)

class PlatformTextCapture(unittest.TestCase):
    def capture(self, samples, rpc_seconds=0):
        # Execute the real observer method with scripted D-Bus replies. GI is
        # unnecessary for this transport race; no fake editor buffer is used.
        import ast
        import time
        from types import SimpleNamespace
        file = Path(__file__).resolve().parents[1] / 'a11y/observe-atspi.py'
        tree = ast.parse(file.read_text())
        observer = next(node for node in tree.body if isinstance(node, ast.ClassDef) and node.name == 'Observer')
        method = next(node for node in observer.body if isinstance(node, ast.FunctionDef) and node.name == 'read_text')
        clock = [0.0]
        self.rpc_timeouts = []
        scope = {'TEXT': 'Text', 'PROPERTIES': 'Properties', 'ACCESSIBLE': 'Accessible',
                 'time': SimpleNamespace(monotonic=lambda: clock[0]),
                 'dbus': SimpleNamespace(Int32=int), 'plain': lambda value: value}
        timeouts = self.rpc_timeouts
        exec(compile(ast.Module(body=[method], type_ignores=[]), str(file), 'exec'), scope)
        class Replies:
            index = -1
            def delay(self, kwargs):
                timeouts.append(kwargs['timeout'])
                clock[0] += rpc_seconds
            def GetInterfaces(self, **kwargs):
                self.delay(kwargs)
                return ['Text']
            def GetText(self, *args, **kwargs):
                self.delay(kwargs)
                self.index += 1
                self.current = samples[min(self.index, len(samples) - 1)]
                return self.current['text']
            def Get(self, *args, **kwargs):
                self.delay(kwargs)
                return self.current['caret']
            def GetNSelections(self, **kwargs):
                self.delay(kwargs)
                return len(self.current['selections'])
            def GetSelection(self, index, **kwargs):
                self.delay(kwargs)
                return self.current['selections'][index]
        replies = Replies()
        owner = SimpleNamespace(read=lambda _: {'interfaces': ['Text']}, interface=lambda *_: replies)
        return scope['read_text'](owner, ('fixture', 'entry'))

    def test_changed_text_reads_preserve_the_raw_race_until_two_equal_samples(self):
        raw = [{'text': 'ab', 'caret': 3, 'selections': []},
               {'text': 'abc', 'caret': 3, 'selections': []},
               {'text': 'abc', 'caret': 3, 'selections': []}]
        observed = self.capture(raw)
        self.assertEqual(observed['text'], 'abc')
        self.assertEqual(observed['caret'], 3)
        self.assertEqual(observed['samples'], raw)

    def test_stable_invalid_offsets_are_not_clamped_or_retried_as_success(self):
        with self.assertRaisesRegex(RuntimeError, 'Stable platform Text offsets are invalid'):
            self.capture([{'text': 'ab', 'caret': 3, 'selections': []}] * 2)

    def test_unstable_text_exhausts_the_explicit_capture_budget(self):
        with self.assertRaisesRegex(RuntimeError, 'did not stabilize'):
            self.capture([{'text': str(index), 'caret': 1, 'selections': []} for index in range(8)])

    def test_each_rpc_uses_remaining_budget_and_late_replies_never_pass(self):
        with self.assertRaisesRegex(RuntimeError, 'deadline expired'):
            self.capture([{'text': 'abc', 'caret': 3, 'selections': []}] * 2, rpc_seconds=0.5)
        self.assertEqual(self.rpc_timeouts, [3.0, 2.5, 2.0, 1.5, 1.0, 0.5])


class PlatformRoleVocabulary(unittest.TestCase):
    def test_numeric_button_role_is_stable_without_relabeling_toggles(self):
        import ast
        file = Path(__file__).resolve().parents[1] / 'a11y/observe-atspi.py'
        tree = ast.parse(file.read_text())
        method = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'canonical_role')
        scope = {}
        exec(compile(ast.Module(body=[method], type_ignores=[]), str(file), 'exec'), scope)
        canonical = scope['canonical_role']
        for native in ('button', 'push button'):
            self.assertEqual(canonical(43, native), 'push button')
        self.assertEqual(canonical(62, 'toggle button'), 'toggle button')
        self.assertEqual(canonical(62, 'button'), 'button')


if __name__ == '__main__':
    unittest.main()
