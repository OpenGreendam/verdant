#!/usr/bin/env python3
"""Run the real input probe in an owned Xvfb/Weston/AT-SPI session.

Dependencies are extracted under --deps-root, or installed by the caller when
--deps-root is /. This script installs no packages and changes no system
services. Every child inherits at most two CPUs and an
8 GiB address-space limit. The caller must serialize resource-intensive runs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import selectors
import signal
import subprocess
import sys
import time
from xml.sax.saxutils import escape

from probe import CASES, CONTROL_CASES


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--deps-root', required=True, type=Path,
                        help='Extracted dependency prefix, or / for installed system packages')
    parser.add_argument('--repo', required=True, type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--worker-binary', required=True, type=Path)
    parser.add_argument('--backend', required=True, choices=('x11', 'wayland'))
    parser.add_argument('--suite', choices=('original', 'controls'), default='original')
    parser.add_argument('--run-dir', required=True, type=Path, help='New, short evidence/socket path')
    parser.add_argument('--python', type=Path, default=Path('/usr/bin/python3'),
                        help='Python matching the selected dbus/GI extension ABI')
    parser.add_argument('probe_options', nargs=argparse.REMAINDER,
                        help='Optional diagnostic probe arguments after --; omit for the complete suite')
    args = parser.parse_args()
    if args.probe_options[:1] == ['--']:
        args.probe_options.pop(0)
    return args


def sha256(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def read_display_number(read_fd, timeout=10):
    """Wait for Xvfb's complete displayfd line, including separate writes."""
    deadline = time.monotonic() + timeout
    data = bytearray()
    with selectors.DefaultSelector() as selector:
        selector.register(read_fd, selectors.EVENT_READ)
        while b'\n' not in data:
            if len(data) >= 64:
                raise RuntimeError('Private Xvfb display number exceeded its protocol budget')
            if not selector.select(max(0, deadline - time.monotonic())):
                raise RuntimeError('Private Xvfb did not publish its complete display number')
            part = os.read(read_fd, 64 - len(data))
            if not part:
                raise RuntimeError('Private Xvfb closed displayfd before its newline')
            data.extend(part)
    if not data.endswith(b'\n') or not data[:-1].isdigit():
        raise RuntimeError('Private Xvfb returned an invalid display number')
    return data[:-1].decode('ascii')


def weston_module_map(deps, library_dirs, major):
    # Do not recursively walk /usr/lib when using installed Ubuntu packages.
    # Both Weston 13 and 14 use name=path entries separated by semicolons.
    modules = {}
    for directory in [deps / 'usr/lib', *library_dirs]:
        for relative in ('weston/*.so', f'libweston-{major}/*.so'):
            for path in sorted(directory.glob(relative)):
                if path.name in modules and modules[path.name] != str(path):
                    raise RuntimeError(f'Ambiguous Weston module {path.name}')
                modules[path.name] = str(path)
    if 'x11-backend.so' not in modules or 'desktop-shell.so' not in modules:
        raise RuntimeError('The selected Weston packages must include the X11 backend and desktop shell')
    return ';'.join(f'{name}={path}' for name, path in sorted(modules.items()))


def weston_major(version, help_text):
    match = re.fullmatch(r'weston (\d+)\.\d+\.\d+(?:[^\r\n]*)', version.strip())
    if not match or int(match[1]) not in (13, 14):
        raise RuntimeError(f'Unsupported Weston version; review its scene graph format first: {version!r}')
    if '--renderer=NAME' not in help_text or 'pixman' not in help_text or '--debug' not in help_text:
        raise RuntimeError('Weston must advertise the reviewed pixman renderer and debug protocol')
    return int(match[1])


def weston_x11_output(log):
    """Use the owned compositor's observed output ID, never guess by size/title."""
    outputs = []
    for line in log.splitlines():
        match = re.search(r'\bx11 output (\d+)x(\d+), window id (\d+)\s*$', line)
        if match:
            width, height, window = map(int, match.groups())
            if min(width, height, window) <= 0:
                raise RuntimeError('Weston reported an invalid X11 output/window ID')
            outputs.append({'width': width, 'height': height, 'window': str(window), 'logLine': line})
    if len(outputs) > 1:
        raise RuntimeError('Expected one observed X11 output from the owned Weston compositor')
    return outputs[0] if outputs else None


def main(args):
    deps, repo = args.deps_root.resolve(strict=True), args.repo.resolve(strict=True)
    binary, worker = args.binary.resolve(strict=True), args.worker_binary.resolve(strict=True)
    python = args.python.resolve(strict=True)
    run = args.run_dir.absolute()
    run.mkdir(parents=True, mode=0o700, exist_ok=False)
    run = run.resolve()
    if len(os.fsencode(run / 'a11y/accessibility.sock')) >= 104:
        raise RuntimeError('Choose a shorter --run-dir for private Unix sockets')
    cpus = sorted(os.sched_getaffinity(0))[:2]
    os.sched_setaffinity(0, cpus)
    address_limit = 8 * 1024 ** 3
    _, hard = resource.getrlimit(resource.RLIMIT_AS)
    if hard != resource.RLIM_INFINITY:
        address_limit = min(address_limit, hard)
    resource.setrlimit(resource.RLIMIT_AS, (address_limit, address_limit))

    def dependency(relative):
        file = deps / relative
        if not file.is_file():
            raise RuntimeError(f'Missing dependency: {file}')
        return str(file)

    xvfb = dependency('usr/bin/Xvfb')
    xdotool = dependency('usr/bin/xdotool')
    library_dirs = sorted(deps.glob('usr/lib/*-linux-gnu'))
    runtime_library_dirs = [*library_dirs, *(path / 'weston' for path in library_dirs if (path / 'weston').is_dir())]
    env = os.environ.copy()
    for key in ('DBUS_SESSION_BUS_ADDRESS', 'AT_SPI_BUS_ADDRESS', 'WAYLAND_DISPLAY', 'WAYLAND_SOCKET', 'XAUTHORITY'):
        env.pop(key, None)
    env['PATH'] = str(deps / 'usr/bin') + os.pathsep + env.get('PATH', '')
    env['LD_LIBRARY_PATH'] = os.pathsep.join([*(str(path) for path in runtime_library_dirs), env.get('LD_LIBRARY_PATH', '')]).rstrip(os.pathsep)
    for directory in ('runtime', 'config', 'cache', 'data', 'font-cache'):
        (run / directory).mkdir(mode=0o700)
    env.update({
        'XDG_RUNTIME_DIR': str(run / 'runtime'), 'XDG_CONFIG_HOME': str(run / 'config'),
        'XDG_CACHE_HOME': str(run / 'cache'), 'XDG_DATA_HOME': str(run / 'data'),
        'XDG_DATA_DIRS': str(deps / 'usr/share'),
        'LIBGL_ALWAYS_SOFTWARE': '1', 'MESA_LOADER_DRIVER_OVERRIDE': 'llvmpipe',
        'LP_NUM_THREADS': '2', 'RAYON_NUM_THREADS': '2', 'OMP_NUM_THREADS': '2',
        'MESA_SHADER_CACHE_DIR': str(run / 'cache/mesa'),
        'XKB_CONFIG_ROOT': str(deps / 'usr/share/X11/xkb'),
        'WESTON_DATA_DIR': str(deps / 'usr/share/weston'),
    })
    font_config = run / 'fonts.conf'
    font_config.write_text('<?xml version="1.0"?><!DOCTYPE fontconfig SYSTEM "fonts.dtd">\n<fontconfig>\n'
                          f'<dir>{escape(str(deps / "usr/share/fonts"))}</dir>\n<dir>/usr/share/fonts</dir>\n'
                          f'<cachedir>{escape(str(run / "font-cache"))}</cachedir>\n'
                          '<config><rescan><int>0</int></rescan></config>\n</fontconfig>\n')
    env['FONTCONFIG_FILE'] = str(font_config)
    icds = sorted((deps / 'usr/share/vulkan/icd.d').glob('lvp_icd*.json'))
    drivers = [path / 'libvulkan_lvp.so' for path in library_dirs if (path / 'libvulkan_lvp.so').is_file()]
    if len(icds) != 1 or len(drivers) != 1:
        raise RuntimeError('Expected one Lavapipe ICD and driver in the selected dependency prefix')
    icd = json.loads(icds[0].read_text())
    icd['ICD']['library_path'] = str(drivers[0])
    private_icd = run / 'lavapipe.json'
    private_icd.write_text(json.dumps(icd, indent=2) + '\n')
    env['VK_DRIVER_FILES'] = env['VK_ICD_FILENAMES'] = str(private_icd)
    record = {'format': 'verdant.isolated-native-run.v1', 'backend': args.backend, 'suite': args.suite,
              'runDirectory': str(run), 'dependencies': str(deps),
              'binarySha256': sha256(binary), 'workerSha256': sha256(worker),
              'cpuAffinity': cpus, 'perProcessAddressSpaceLimitBytes': address_limit,
              'threads': {'lavapipe': 2, 'rayon': 2, 'openmp': 2},
              'probeOptions': args.probe_options, 'processes': [], 'ok': False}
    processes, logs = [], []

    def save():
        (run / 'harness.json').write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n')

    def start(name, command, **kwargs):
        log = (run / f'{name}.log').open('w')
        logs.append(log)
        child = subprocess.Popen(command, env=env, stdout=log, stderr=log,
                                 start_new_session=True, **kwargs)
        processes.append((name, child))
        record['processes'].append({'name': name, 'pid': child.pid, 'ownedProcessGroup': child.pid,
                                    'command': command})
        save()
        return child

    def xdo(*command):
        return subprocess.run([xdotool, *map(str, command)], env=env, capture_output=True,
                              text=True, timeout=5)

    def wait_for(operation, predicate, description, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = operation()
            if predicate(value):
                return value
            if any(child.poll() is not None for _, child in processes):
                raise RuntimeError(f'Owned display process exited while waiting for {description}; inspect logs')
            time.sleep(0.05)
        raise RuntimeError(f'Timed out waiting for {description}')

    try:
        read_fd, write_fd = os.pipe()
        try:
            start('xvfb', [xvfb, '-displayfd', str(write_fd), '-screen', '0', '1920x1200x24',
                           '-nolisten', 'tcp', '-ac', '-noreset'], pass_fds=(write_fd,))
        finally:
            os.close(write_fd)
        try:
            display_number = read_display_number(read_fd)
        finally:
            os.close(read_fd)
        env['DISPLAY'] = ':' + display_number
        wait_for(lambda: xdo('getdisplaygeometry'), lambda result: result.returncode == 0,
                 'private X11 display readiness')
        record['inputDisplay'] = env['DISPLAY']
        probe = [str(python), str(repo / 'tools/native-input/probe.py'),
                 '--binary', str(binary), '--worker-binary', str(worker), '--repo', str(repo),
                 '--output-dir', str(run / 'evidence'), '--backend', args.backend, '--suite', args.suite,
                 '--input-display', env['DISPLAY'], '--xdotool', xdotool]
        if args.backend == 'wayland':
            weston = dependency('usr/bin/weston')
            debug = dependency('usr/bin/weston-debug')
            shell = dependency('usr/libexec/weston-desktop-shell')
            version = subprocess.run([weston, '--version'], env=env, check=True,
                                     capture_output=True, text=True, timeout=5).stdout.strip()
            help_result = subprocess.run([weston, '--help'], env=env, check=True,
                                         capture_output=True, text=True, timeout=5)
            help_text = help_result.stdout + help_result.stderr
            (run / 'weston-help.txt').write_text(help_text)
            (run / 'weston-version.txt').write_text(version + '\n')
            major = weston_major(version, help_text)
            env['WESTON_MODULE_MAP'] = weston_module_map(deps, library_dirs, major)
            record['weston'] = {'version': version, 'major': major,
                                'helpEvidence': 'weston-help.txt', 'moduleMap': env['WESTON_MODULE_MAP']}
            config = run / 'weston.ini'
            config.write_text('[shell]\nclient=' + shell + '\npanel-position=none\n'
                              'background-color=0xff25303b\nlocking=false\nanimation=none\n'
                              'close-animation=none\nstartup-animation=none\nfocus-animation=none\n'
                              '[input-method]\npath=' + dependency('usr/libexec/weston-keyboard') + '\n')
            env['WAYLAND_DISPLAY'] = 'wayland-verdant-audit'
            start('weston', [weston, '--backend=x11', '--renderer=pixman', '--shell=desktop-shell.so',
                             '--debug', '--config=' + str(config), '--socket=' + env['WAYLAND_DISPLAY'],
                             '--idle-time=0', '--scale=1', '--width=1600', '--height=1000',
                             '--log=' + str(run / 'weston-server.log')])
            wait_for(lambda: (run / 'runtime' / env['WAYLAND_DISPLAY']).is_socket(), bool,
                     'private Wayland socket')
            server_log = run / 'weston-server.log'
            output = wait_for(lambda: weston_x11_output(server_log.read_text() if server_log.exists() else ''),
                              lambda value: value is not None, 'the owned compositor X11 output record')
            geometry = wait_for(lambda: xdo('getwindowgeometry', '--shell', output['window']),
                                lambda result: result.returncode == 0, 'the observed Weston X11 window geometry')
            fields = dict(line.split('=', 1) for line in geometry.stdout.splitlines() if '=' in line)
            actual = {name: int(fields[name]) for name in ('X', 'Y', 'WIDTH', 'HEIGHT')}
            if (actual['WIDTH'], actual['HEIGHT']) != (output['width'], output['height']):
                raise RuntimeError('Observed Weston X11 window geometry differs from its output record')
            record['outerWindow'] = output['window']
            record['outerWindowEvidence'] = {**output, 'evidence': 'weston-server.log', 'x11Geometry': actual}
            probe += ['--outer-window', output['window'], '--wayland-windowed', '--weston-debug', debug]
        probe += args.probe_options
        runner = [str(python), str(repo / 'tools/a11y/run-isolated.py'), '--deps-root', str(deps),
                  '--run-dir', str(run / 'a11y'), '--', *probe]
        child = start('probe', runner)
        print(json.dumps({'running': True, 'backend': args.backend, 'runDirectory': str(run),
                          'probePid': child.pid}), flush=True)
        status = child.wait()
        record['exitCode'] = status
        result = json.loads((run / 'evidence/result.json').read_text()) if (run / 'evidence/result.json').exists() else None
        record['ok'] = status == 0 and result is not None and result.get('ok') is True
        if record['ok'] and not args.probe_options:
            cases = CASES if args.suite == 'original' else CONTROL_CASES
            complete = result.get('skipSetValue') is False and result.get('cases') == [
                {'name': name, 'status': 'passed'} for name in cases]
            if not complete:
                record['ok'] = False
                record['error'] = 'An unfiltered run must pass every case in its declared suite'
        save()
        print(json.dumps({'ok': record['ok'], 'backend': args.backend, 'runDirectory': str(run),
                          'cases': result.get('cases') if result else None,
                          'error': result.get('error') if result else 'No probe result; inspect probe.log'}, ensure_ascii=False), flush=True)
        return 0 if record['ok'] else 1
    except BaseException as error:
        record['error'] = str(error)
        save()
        raise
    finally:
        for name, child in reversed(processes):
            try:
                os.killpg(child.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                child.wait(timeout=8)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=4)
            for item in record['processes']:
                if item['pid'] == child.pid:
                    item['returnCodeAfterCleanup'] = child.returncode
        for log in logs:
            log.close()
        save()


if __name__ == '__main__':
    def interrupted(signum, _frame):
        raise InterruptedError(f'Isolated native harness received signal {signum}')

    signal.signal(signal.SIGTERM, interrupted)
    try:
        sys.exit(main(arguments()))
    except (OSError, RuntimeError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        print(f'Isolated native harness failed: {error}', file=sys.stderr)
        sys.exit(1)
