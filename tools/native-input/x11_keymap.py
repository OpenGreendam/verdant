"""Prepare persistent Unicode key symbols on a harness-owned X11 display.

xdotool can type an unmapped symbol by briefly rebinding an unused X11 key.
That temporary core mapping need not reach a nested Wayland compositor before
the key arrives. Install the required symbols before starting the compositor
and application, then keep them for the lifetime of this private Xvfb server.
"""

import ctypes
import hashlib
import json
from pathlib import Path
import re
import subprocess


RULE_FIELDS = ('rules', 'model', 'layout', 'variant', 'options')
PRIVATE_LAYOUT = 'verdant_controls'


def _component_body(keymap, kind):
    """Extract an xkbcomp component, ignoring braces inside strings/comments."""
    masked = re.sub(r'"(?:\\.|[^"\\])*"|/\*.*?\*/|//[^\n]*',
                    lambda match: ' ' * len(match[0]), keymap, flags=re.DOTALL)
    starts = list(re.finditer(r'\b' + re.escape(kind) + r'\s*\{', masked))
    if len(starts) != 1:
        raise RuntimeError(f'Expected one actual {kind} component in the observed XKB map')
    start = starts[0].end()
    depth = 1
    for index in range(start, len(masked)):
        depth += (masked[index] == '{') - (masked[index] == '}')
        if depth == 0:
            return keymap[start:index]
    raise RuntimeError(f'Unclosed {kind} component in the observed XKB map')


def prepare_named_keymap(keymap, run):
    """Give Weston a rules-resolvable copy of the actual server components."""
    root = run / 'x11-keymap-root'
    components = {'keycodes': 'xkb_keycodes', 'types': 'xkb_types',
                  'compat': 'xkb_compatibility', 'symbols': 'xkb_symbols'}
    bodies = {directory: _component_body(keymap, kind)
              for directory, kind in components.items()}
    root.mkdir(mode=0o700)
    for directory, kind in components.items():
        (root / directory).mkdir(mode=0o700)
        (root / directory / PRIVATE_LAYOUT).write_text(
            f'default {kind} "basic" {{' + bodies[directory] + '};\n')
    (root / 'rules').mkdir(mode=0o700)
    (root / 'rules' / PRIVATE_LAYOUT).write_text(''.join(
        f'! {"layout" if component == "symbols" else "model"} = {component}\n'
        f'  {PRIVATE_LAYOUT} = {PRIVATE_LAYOUT}(basic)\n'
        for component in components))
    return root, dict(zip(RULE_FIELDS, (PRIVATE_LAYOUT, PRIVATE_LAYOUT, PRIVATE_LAYOUT, '', '')))


def _library(name, env):
    # The harness can use an extracted dependency prefix. Prefer that exact
    # prefix over a similarly named host installation when loading its ABI.
    for directory in env.get('LD_LIBRARY_PATH', '').split(':'):
        if directory and (Path(directory) / name).is_file():
            return ctypes.CDLL(str((Path(directory) / name).resolve()))
    return ctypes.CDLL(name)


def compile_named_keymap(root, names, env):
    """Actually use xkbcommon's same RMLVO entry point as Weston 13/14."""
    class RuleNames(ctypes.Structure):
        _fields_ = [(name, ctypes.c_char_p) for name in RULE_FIELDS]

    xkb = _library('libxkbcommon.so.0', env)
    xkb.xkb_context_new.argtypes = [ctypes.c_int]
    xkb.xkb_context_new.restype = ctypes.c_void_p
    xkb.xkb_context_include_path_append.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    xkb.xkb_context_include_path_append.restype = ctypes.c_int
    xkb.xkb_keymap_new_from_names.argtypes = [ctypes.c_void_p, ctypes.POINTER(RuleNames), ctypes.c_int]
    xkb.xkb_keymap_new_from_names.restype = ctypes.c_void_p
    xkb.xkb_keymap_get_as_string.argtypes = [ctypes.c_void_p, ctypes.c_int]
    xkb.xkb_keymap_get_as_string.restype = ctypes.c_void_p
    xkb.xkb_keymap_unref.argtypes = [ctypes.c_void_p]
    xkb.xkb_keymap_unref.restype = None
    xkb.xkb_context_unref.argtypes = [ctypes.c_void_p]
    xkb.xkb_context_unref.restype = None
    libc = ctypes.CDLL(None)
    libc.free.argtypes = [ctypes.c_void_p]
    libc.free.restype = None
    # NO_DEFAULT_INCLUDES: this proves the retained private files resolve the
    # names; a host user/system layout cannot accidentally satisfy the check.
    context = xkb.xkb_context_new(1)
    if not context:
        raise RuntimeError('Could not create the native XKB names compiler context')
    keymap = serialized = None
    try:
        if not xkb.xkb_context_include_path_append(context, str(root).encode()):
            raise RuntimeError('The private XKB include root was rejected')
        requested = RuleNames(*(names[name].encode() for name in RULE_FIELDS))
        keymap = xkb.xkb_keymap_new_from_names(context, ctypes.byref(requested), 0)
        if not keymap:
            raise RuntimeError('The observed XKB rules names did not compile')
        serialized = xkb.xkb_keymap_get_as_string(keymap, 1)
        if not serialized:
            raise RuntimeError('Could not serialize the keymap compiled from observed names')
        return ctypes.string_at(serialized).decode()
    finally:
        if serialized:
            libc.free(serialized)
        if keymap:
            xkb.xkb_keymap_unref(keymap)
        xkb.xkb_context_unref(context)


def verify_rules_names(observed, expected):
    required = b''.join(expected[name].encode() + b'\0' for name in RULE_FIELDS)
    if observed != required:
        raise RuntimeError('The X11 rules names differ from the private keyboard layout')
    return dict(zip(RULE_FIELDS, (value.decode() for value in observed.split(b'\0')[:-1])))


def install_rules_names(env, names):
    """Set and read back only the newly owned Xvfb's native root property."""
    x11 = _library('libX11.so.6', env)
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
    x11.XDefaultRootWindow.restype = ctypes.c_ulong
    x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
    x11.XInternAtom.restype = ctypes.c_ulong
    x11.XChangeProperty.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong,
                                   ctypes.c_ulong, ctypes.c_int, ctypes.c_int,
                                   ctypes.c_void_p, ctypes.c_int]
    x11.XGetWindowProperty.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong,
                                     ctypes.c_long, ctypes.c_long, ctypes.c_int, ctypes.c_ulong,
                                     ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int),
                                     ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
                                     ctypes.POINTER(ctypes.c_void_p)]
    x11.XGetWindowProperty.restype = ctypes.c_int
    x11.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
    x11.XFree.argtypes = [ctypes.c_void_p]
    x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
    display = x11.XOpenDisplay(env['DISPLAY'].encode())
    if not display:
        raise RuntimeError('Could not open the harness-owned Xvfb to set its keyboard rules')
    try:
        root = x11.XDefaultRootWindow(display)
        atom = x11.XInternAtom(display, b'_XKB_RULES_NAMES', 0)

        def read():
            actual, form, count, left = ctypes.c_ulong(), ctypes.c_int(), ctypes.c_ulong(), ctypes.c_ulong()
            data = ctypes.c_void_p()
            status = x11.XGetWindowProperty(display, root, atom, 0, 1024, 0, 31,
                                           ctypes.byref(actual), ctypes.byref(form), ctypes.byref(count),
                                           ctypes.byref(left), ctypes.byref(data))
            try:
                if status or left.value or (actual.value and (actual.value != 31 or form.value != 8)):
                    raise RuntimeError('Could not read the complete native XKB STRING/8 property')
                return ctypes.string_at(data, count.value) if actual.value else b''
            finally:
                if data:
                    x11.XFree(data)

        before = read()
        payload = b''.join(names[name].encode() + b'\0' for name in RULE_FIELDS)
        buffer = ctypes.create_string_buffer(payload)
        x11.XChangeProperty(display, root, atom, 31, 8, 0, buffer, len(payload))
        x11.XSync(display, 0)
        observed = read()
        actual_names = verify_rules_names(observed, names)
        return actual_names, {'atom': '_XKB_RULES_NAMES', 'type': 'STRING', 'format': 8,
                              'beforeHex': before.hex(), 'observedHex': observed.hex(), 'names': actual_names}
    finally:
        x11.XCloseDisplay(display)


def prepare_unicode_keymap(keymap, inputs):
    """Add only previously unused XKB keys, retaining every existing binding."""
    characters = sorted({char for text in inputs for char in text if ord(char) > 127}, key=ord)
    if any(0xD800 <= ord(char) <= 0xDFFF for char in characters):
        raise RuntimeError('Unicode keyboard input contains a surrogate code point')
    if not characters:
        return keymap, []
    keycodes = [(name, int(code)) for name, code in re.findall(
        r'^\s*<([^>]+)>\s*=\s*(\d+)\s*;', keymap, re.MULTILINE)]
    if len({name for name, _ in keycodes}) != len(keycodes) or len({code for _, code in keycodes}) != len(keycodes):
        raise RuntimeError('Ambiguous XKB keycode declarations')
    aliases = dict(re.findall(r'\balias\s+<([^>]+)>\s*=\s*<([^>]+)>\s*;', keymap))
    used = set(re.findall(r'\bkey\s+<([^>]+)>\s*\{', keymap))
    for modifier in re.findall(r'\bmodifier_map\s+\w+\s*\{([^}]+)\}', keymap):
        used.update(re.findall(r'<([^>]+)>', modifier))
    for name in list(used):
        visited = set()
        while name in aliases:
            if name in visited:
                raise RuntimeError('Cyclic XKB key aliases')
            visited.add(name)
            name = aliases[name]
            used.add(name)
    # XTest has an eight-bit X11 keycode. Avoid keycode 8 (Wayland key 0),
    # and never replace a symbol or modifier already declared in this layout.
    available = sorted(((name, code) for name, code in keycodes if 9 <= code <= 255 and name not in used),
                       key=lambda item: item[1], reverse=True)
    if len(available) < len(characters):
        raise RuntimeError('Not enough unused XKB keys for the Unicode control journey')
    mappings = [{'character': char, 'codepoint': ord(char), 'keysym': f'U{ord(char):04X}',
                 'keyName': name, 'x11Keycode': code, 'waylandKeycode': code - 8}
                for char, (name, code) in zip(characters, available)]
    symbols = re.search(r'\bxkb_symbols\s+(?:"[^"]*"\s*)?\{', keymap)
    if symbols is None:
        raise RuntimeError('Missing XKB symbols section')
    additions = ''.join('\n    key <' + entry['keyName'] + '> { type[Group1]="ONE_LEVEL", '
                        'symbols[Group1]=[ ' + entry['keysym'] + ' ] };' for entry in mappings)
    return keymap[:symbols.end()] + additions + keymap[symbols.end():], mappings


def verify_unicode_keymap(keymap, mappings):
    """Check the keymap actually read back from Xvfb, not the requested text."""
    for entry in mappings:
        codes = re.findall(r'^\s*<' + re.escape(entry['keyName']) + r'>\s*=\s*(\d+)\s*;',
                           keymap, re.MULTILINE)
        if len(codes) != 1 or int(codes[0]) != entry['x11Keycode'] \
                or not 9 <= int(codes[0]) <= 255 or int(codes[0]) - 8 != entry['waylandKeycode']:
            raise RuntimeError('Unicode XKB keycode differs from the recorded native input mapping')
        blocks = re.findall(r'\bkey\s+<' + re.escape(entry['keyName']) + r'>\s*\{(.*?)\};', keymap, re.DOTALL)
        if len(blocks) != 1:
            raise RuntimeError('Unicode XKB binding is missing or ambiguous after installation')
        symbols = re.findall(r'\[\s*U([0-9a-fA-F]{4,8})\s*\]', blocks[0])
        if len(symbols) != 1 or int(symbols[0], 16) != entry['codepoint']:
            raise RuntimeError('Unicode XKB binding differs from the required character')


def install_unicode_keymap(xkbcomp, env, run, inputs):
    """Called only after main creates its Xvfb, before any Weston/app process."""
    before = subprocess.run([xkbcomp, '-xkb', env['DISPLAY'], '-'], env=env, check=True,
                            capture_output=True, text=True, timeout=5)
    source, mappings = prepare_unicode_keymap(before.stdout, inputs)
    original = run / 'x11-keymap-before.xkb'
    prepared = run / 'x11-keymap-prepared.xkb'
    observed = run / 'x11-keymap-observed.xkb'
    original.write_text(before.stdout)
    prepared.write_text(source)
    loaded = subprocess.run([xkbcomp, '-w', '0', str(prepared), env['DISPLAY']], env=env,
                            capture_output=True, text=True, timeout=5)
    (run / 'x11-keymap-load.log').write_text(loaded.stdout + loaded.stderr)
    loaded.check_returncode()
    after = subprocess.run([xkbcomp, '-xkb', env['DISPLAY'], '-'], env=env, check=True,
                           capture_output=True, text=True, timeout=5)
    observed.write_text(after.stdout)
    verify_unicode_keymap(after.stdout, mappings)
    # Weston 13's X11 backend reconstructs a keymap from _XKB_RULES_NAMES;
    # successful server-map readback alone does not verify that consumer path.
    root, requested_names = prepare_named_keymap(after.stdout, run)
    names, property_evidence = install_rules_names(env, requested_names)
    from_names = compile_named_keymap(root, names, env)
    verify_unicode_keymap(from_names, mappings)
    named = run / 'x11-keymap-from-names.xkb'
    named.write_text(from_names)
    rules_record = run / 'x11-keymap-rules.json'
    rules_record.write_text(json.dumps(property_evidence, indent=2) + '\n')
    # All descendants, including Weston, resolve these same private names.
    env['XKB_CONFIG_ROOT'] = str(root)
    return {'scope': 'owned Xvfb only; installed before Weston and Verdant', 'display': env['DISPLAY'],
            'transport': 'XTest key events using persistent XKB symbols', 'mappings': mappings,
            'before': original.name, 'prepared': prepared.name, 'observed': observed.name,
            'observedSha256': hashlib.sha256(after.stdout.encode()).hexdigest(),
            'rulesNames': rules_record.name, 'includeRoot': root.name,
            'fromNames': named.name, 'fromNamesSha256': hashlib.sha256(from_names.encode()).hexdigest(),
            'namedFiles': {str(path.relative_to(run)): hashlib.sha256(path.read_bytes()).hexdigest()
                           for path in sorted(root.rglob('*')) if path.is_file()}}
