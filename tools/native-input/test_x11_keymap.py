"""Offline checks for private-display keymap admission, not native UI results."""

import unittest

from x11_keymap import prepare_unicode_keymap, verify_unicode_keymap


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


if __name__ == '__main__':
    unittest.main()
