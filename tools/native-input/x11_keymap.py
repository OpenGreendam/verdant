"""Prepare persistent Unicode key symbols on a harness-owned X11 display.

xdotool can type an unmapped symbol by briefly rebinding an unused X11 key.
That temporary core mapping need not reach a nested Wayland compositor before
the key arrives. Install the required symbols before starting the compositor
and application, then keep them for the lifetime of this private Xvfb server.
"""

import hashlib
import re
import subprocess


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
    return {'scope': 'owned Xvfb only; installed before Weston and Verdant', 'display': env['DISPLAY'],
            'transport': 'XTest key events using persistent XKB symbols', 'mappings': mappings,
            'before': original.name, 'prepared': prepared.name, 'observed': observed.name,
            'observedSha256': hashlib.sha256(after.stdout.encode()).hexdigest()}
