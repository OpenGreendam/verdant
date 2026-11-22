#!/usr/bin/env python3
"""Offline selector/trace regressions; these do not execute a native window."""
import unittest

from probe import nodes_in_window, wayland_contact_evidence, weston_surface_from_scene


SCENE = """Weston scene graph at 262750.759224263:
Layer 5:
    View 0 (role xdg_toplevel, PID 236188, surface ID 15, top-level window 'Verdant 翠绿' of verdant, 0x123):
        position: (103, 6) -> (1583, 926)
        SHM buffer
            width: 1480, height: 920
    View 1 (role xdg_toplevel, PID 236188, surface ID 29, top-level window 'Verdant Synth — Verdant' of verdant, 0x456):
        position: (400, 140) -> (1200, 860)
        SHM buffer
            width: 800, height: 720
    View 2 (role xdg_toplevel, PID 999999, surface ID 29, top-level window 'Verdant Synth — Verdant' of verdant, 0x789):
        position: (0, 0) -> (800, 720)
        SHM buffer
            width: 800, height: 720
"""


class ProbeHelpers(unittest.TestCase):
    def test_identical_names_remain_bound_to_their_observed_frame(self):
        main, native = [':1.2', '/frame/1'], [':1.2', '/frame/2']
        snapshot = {'nodes': [
            {'name': '音量', 'windowReference': main, 'value': 0.53},
            {'name': '音量', 'windowReference': native, 'value': 0.83},
            {'name': '音量', 'windowReference': None, 'value': 0.0},
        ]}
        self.assertEqual([node['value'] for node in nodes_in_window(snapshot, tuple(native))], [0.83])
        self.assertEqual(nodes_in_window(snapshot, [':1.2', '/closed-frame']), [])

    def test_scene_selects_title_and_pid_instead_of_largest_window(self):
        result = weston_surface_from_scene(SCENE, 236188, 'Verdant Synth — Verdant', 'scene-02.txt')
        self.assertEqual([result[key] for key in ('x', 'y', 'width', 'height')], [400, 140, 800, 720])
        self.assertEqual(result['surfaceId'], 29)
        self.assertEqual(result['evidence'], 'scene-02.txt')
        self.assertIsNone(weston_surface_from_scene(SCENE, 236188, 'Missing window'))
        with self.assertRaisesRegex(RuntimeError, 'Multiple native top-levels'):
            weston_surface_from_scene(SCENE, 236188)

    def test_duplicate_titles_and_scaled_buffers_are_rejected(self):
        duplicate = SCENE.replace('PID 999999', 'PID 236188')
        with self.assertRaisesRegex(RuntimeError, 'Multiple native top-levels'):
            weston_surface_from_scene(duplicate, 236188, 'Verdant Synth — Verdant')
        scaled = SCENE.replace('width: 800, height: 720', 'width: 1600, height: 1440')
        with self.assertRaisesRegex(RuntimeError, 'unsupported transform'):
            weston_surface_from_scene(scaled, 236188, 'Verdant Synth — Verdant')

    def test_native_contact_distinguishes_the_observed_escape_regression(self):
        held = """[1] wl_pointer#14.button(71, 47294821, 272, 1)
[2] wl_keyboard#13.key(76, 47295872, 1, 1)
[3] wl_keyboard#13.key(77, 47295902, 1, 0)
[4] wl_pointer#14.motion(47295950, -20.00000000, 460.00000000)
[5] wl_pointer#14.button(74, 47295990, 272, 0)
"""
        result = wayland_contact_evidence(held, [1480, 920])
        self.assertTrue(all(event['pointerHeld'] for event in result['escapeKeyEvents']))
        self.assertEqual(result['outsideReleases'][0]['position'], [-20.0, 460.0])
        released_first = held.replace('[2]', '[1.5] wl_pointer#14.button(73, 47295870, 272, 0)\n[2]')
        self.assertFalse(any(event['pointerHeld'] for event in
                             wayland_contact_evidence(released_first, [1480, 920])['escapeKeyEvents']))


if __name__ == '__main__':
    unittest.main()
