"""Offline checks for private-display keymap admission, not native UI results."""

from pathlib import Path
import tempfile
import unittest

from x11_keymap import (compile_named_keymap, prepare_named_keymap,
                        prepare_unicode_keymap, verify_rules_names, verify_unicode_keymap)


KEYMAP = '''xkb_keymap {
xkb_keycodes "test" {
    minimum = 8;
    maximum = 255;
    <LOW> = 8;
    <AC01> = 38;
    <LFSH> = 50;
    <FREE> = 90;
    <ALIA> = 200;
    <OPEN> = 201;
    <HIGH> = 255;
    alias <USED> = <ALIA>;
};
xkb_symbols "test" {
    key <AC01> { [ a, A ] };
    key <USED> { [ b, B ] };
    modifier_map Shift { <LFSH> };
};
};
'''

# Complete native components for a real libxkbcommon names compilation. The
# existing parser-only fixture above deliberately stays unchanged.
NATIVE_KEYMAP = KEYMAP.replace('xkb_symbols "test" {', '''xkb_types "test" {
    type "ONE_LEVEL" {
        modifiers = None;
        level_name[Level1] = "Any } braces in names are not syntax";
    };
    type "TWO_LEVEL" {
        modifiers = Shift;
        map[Shift] = Level2;
        level_name[Level1] = "Base";
        level_name[Level2] = "Shift";
    };
};
xkb_compatibility "test" {
};
xkb_symbols "test" {''').replace('key <AC01>', 'key <LFSH> { [ Shift_L ] };\n    key <AC01>') \
    .replace('key <AC01> {', 'key <AC01> { type="TWO_LEVEL",') \
    .replace('key <USED> {', 'key <USED> { type="TWO_LEVEL",')


class PrivateKeyboardMapping(unittest.TestCase):
    def test_unicode_uses_free_keys_without_replacing_symbols_aliases_or_modifiers(self):
        prepared, mappings = prepare_unicode_keymap(KEYMAP, ['音😀AB', '音'])
        self.assertEqual([item['character'] for item in mappings], ['音', '😀'])
        self.assertEqual([item['x11Keycode'] for item in mappings], [255, 201])
        self.assertEqual([item['waylandKeycode'] for item in mappings], [247, 193])
        for line in KEYMAP.splitlines():
            self.assertIn(line, prepared)
        verify_unicode_keymap(prepared, mappings)

    def test_insufficient_free_keys_cannot_clobber_an_existing_binding(self):
        with self.assertRaisesRegex(RuntimeError, 'Not enough unused'):
            prepare_unicode_keymap(KEYMAP, ['音😀乐😺'])

    def test_readback_needs_the_required_symbol_on_the_exact_key(self):
        prepared, mappings = prepare_unicode_keymap(KEYMAP, ['音😀'])
        for invalid in [prepared.replace('U97F3', 'U4E50'),
                        prepared.replace('<HIGH> = 255', '<HIGH> = 254'),
                        prepared.replace('key <HIGH>', 'key <MISSING>'),
                        prepared + '\nkey <HIGH> { [ U97F3 ] };']:
            with self.subTest(invalid=invalid):
                with self.assertRaises(RuntimeError):
                    verify_unicode_keymap(invalid, mappings)

    def test_ambiguous_keycodes_and_alias_cycles_fail_before_any_install(self):
        with self.assertRaisesRegex(RuntimeError, 'Ambiguous'):
            prepare_unicode_keymap(KEYMAP.replace('<FREE> = 90', '<FREE> = 38'), ['音'])
        cyclic = KEYMAP.replace('alias <USED> = <ALIA>;',
                                'alias <USED> = <ALIA>; alias <ALIA> = <USED>;')
        with self.assertRaisesRegex(RuntimeError, 'Cyclic'):
            prepare_unicode_keymap(cyclic, ['音'])

    def test_ascii_needs_no_keymap_mutation_and_surrogates_are_not_keys(self):
        self.assertEqual(prepare_unicode_keymap(KEYMAP, ['AB']), (KEYMAP, []))
        with self.assertRaisesRegex(RuntimeError, 'surrogate'):
            prepare_unicode_keymap(KEYMAP, ['\ud800'])

    def test_actual_names_compilation_preserves_unicode_keycodes_and_existing_bindings(self):
        complete = NATIVE_KEYMAP.replace('<HIGH> = 255;', '<HIGH> = 255;\n' + ''.join(
            f'    <U{code}> = {code};\n' for code in range(202, 208)))
        prepared, mappings = prepare_unicode_keymap(complete, ['音😀AB', '音😀乐', '音😺乐', '不应保存'])
        self.assertEqual(len(mappings), 8)
        with tempfile.TemporaryDirectory() as directory:
            root, names = prepare_named_keymap(prepared, Path(directory))
            observed = compile_named_keymap(root, names, {})
        verify_unicode_keymap(observed, mappings)
        self.assertRegex(observed, r'key\s+<AC01>\s*\{[^}]*\ba\s*,\s*A\b')
        self.assertRegex(observed, r'key\s+<ALIA>\s*\{[^}]*\bb\s*,\s*B\b')
        self.assertIn('modifier_map Shift { <LFSH> };', observed)

    def test_server_readback_passes_but_names_without_unicode_are_rejected(self):
        # The #11 gap: reading the modified Xvfb map succeeds even when the
        # compositor still compiles its original layout from rules names.
        server_observed, mappings = prepare_unicode_keymap(NATIVE_KEYMAP, ['音😀'])
        verify_unicode_keymap(server_observed, mappings)
        with tempfile.TemporaryDirectory() as directory:
            root, names = prepare_named_keymap(NATIVE_KEYMAP, Path(directory))
            compositor_observed = compile_named_keymap(root, names, {})
        with self.assertRaisesRegex(RuntimeError, 'missing|differs'):
            verify_unicode_keymap(compositor_observed, mappings)

    def test_names_cannot_silently_fall_back_to_an_unrelated_system_layout(self):
        prepared, _ = prepare_unicode_keymap(NATIVE_KEYMAP, ['音😀'])
        with tempfile.TemporaryDirectory() as directory:
            root, names = prepare_named_keymap(prepared, Path(directory))
            with self.assertRaisesRegex(RuntimeError, 'did not compile'):
                compile_named_keymap(root, {**names, 'rules': 'evdev'}, {})

    def test_rules_property_readback_requires_all_five_exact_native_fields(self):
        names = {'rules': 'verdant_controls', 'model': 'verdant_controls',
                 'layout': 'verdant_controls', 'variant': '', 'options': ''}
        payload = b'verdant_controls\0verdant_controls\0verdant_controls\0\0\0'
        self.assertEqual(verify_rules_names(payload, names), names)
        for invalid in (payload[:-1], payload + b'\0', payload.replace(b'verdant_controls', b'us', 1)):
            with self.subTest(observed=invalid):
                with self.assertRaisesRegex(RuntimeError, 'rules names differ'):
                    verify_rules_names(invalid, names)

    def test_incomplete_components_fail_before_creating_the_private_layout(self):
        with tempfile.TemporaryDirectory() as directory:
            run = Path(directory)
            with self.assertRaisesRegex(RuntimeError, 'one actual xkb_types'):
                prepare_named_keymap(KEYMAP, run)
            self.assertFalse((run / 'x11-keymap-root').exists())


if __name__ == '__main__':
    unittest.main()
