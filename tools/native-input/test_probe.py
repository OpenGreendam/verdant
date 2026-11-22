#!/usr/bin/env python3
"""Offline selector/trace regressions; these do not execute a native window."""
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock

from probe import Probe, nodes_in_window, wayland_contact_evidence, weston_surface_from_scene


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


class PresetActionSelected(Exception):
    """Stop the offline case at the real accessibility transport boundary."""


class ProbeHelpers(unittest.TestCase):
    def preset_probe(self, controls):
        main, native = [':1.3', '/frame/0'], [':1.3', '/frame/3']
        snapshot = {'nodes': [
            {'reference': main, 'name': 'project', 'role': 'frame'},
            {'reference': native, 'name': 'Verdant Synth — Verdant', 'role': 'frame'},
            *controls,
        ]}
        probe = object.__new__(Probe)
        probe.binary = Path('verdant-gui')
        probe.observer = SimpleNamespace(snapshot=lambda _: snapshot)
        probe.args = SimpleNamespace(timeout=0)
        probe.process = None
        probe.frame = None
        probe.main_frame_reference = tuple(main)
        probe.active_frame_reference = tuple(main)
        before = {'gainDb': -8.0, 'revision': 12, 'sha256': 'unchanged'}
        probe.persisted = lambda: before
        probe.open_native_window = lambda: setattr(probe, 'active_frame_reference', tuple(native))
        probe.start_drag = lambda: (before, (0, 0), 1.0)
        probe.record = Mock()
        # Use the actual case, snapshot, nodes, node and wait methods. Only OS
        # setup is substituted; stop before sending any accessibility action.
        probe.advertised_action = Mock(side_effect=PresetActionSelected)
        return probe

    def test_native_preset_case_selects_its_frame_and_combo_role(self):
        main, native = [':1.3', '/frame/0'], [':1.3', '/frame/3']
        selected = {'reference': [':1.3', '/native/preset'], 'name': '预设',
                    'role': 'combo box', 'windowReference': native}
        probe = self.preset_probe([
            {'reference': [':1.3', '/main/preset'], 'name': '预设', 'role': 'combo box', 'windowReference': main},
            {'reference': [':1.3', '/main/custom'], 'name': '自定义', 'role': 'combo box', 'windowReference': main},
            {'reference': [':1.3', '/native/button'], 'name': '预设', 'role': 'push button', 'windowReference': native},
            {'reference': [':1.3', '/unowned/preset'], 'name': '预设', 'role': 'combo box', 'windowReference': None},
            selected,
        ])
        with self.assertRaises(PresetActionSelected):
            probe.native_preset_cancel()
        probe.advertised_action.assert_called_once_with(selected)
        self.assertEqual(probe.node('预设', 'combo box')['reference'], selected['reference'])
        self.assertEqual(len(probe.nodes('预设')), 2)
        self.assertEqual(probe.nodes('自定义', 'combo box'), [])

    def test_native_preset_case_rejects_missing_wrong_role_and_legacy_targets(self):
        main, native = [':1.3', '/frame/0'], [':1.3', '/frame/3']
        for native_controls in [
            [],
            [{'name': '预设', 'role': 'push button', 'windowReference': native}],
            [{'name': '自定义', 'role': 'combo box', 'windowReference': native}],
        ]:
            with self.subTest(native_controls=native_controls):
                probe = self.preset_probe([
                    {'name': '预设', 'role': 'combo box', 'windowReference': main},
                    {'name': '自定义', 'role': 'combo box', 'windowReference': main},
                    *native_controls,
                ])
                with self.assertRaisesRegex(RuntimeError, "Timed out waiting for '预设' in AT-SPI"):
                    probe.native_preset_cancel()
                probe.advertised_action.assert_not_called()

    def test_native_preset_case_rejects_ambiguous_native_combos(self):
        native = [':1.3', '/frame/3']
        probe = self.preset_probe([
            {'reference': [':1.3', f'/native/preset/{index}'], 'name': '预设',
             'role': 'combo box', 'windowReference': native}
            for index in range(2)
        ])
        with self.assertRaisesRegex(RuntimeError, "Ambiguous accessible target '预设': 2 matches"):
            probe.native_preset_cancel()
        probe.advertised_action.assert_not_called()

    def test_native_close_selects_button_among_same_frame_radio_names(self):
        main, native = [':1.3', '/frame/0'], [':1.3', '/frame/1']
        snapshot = {'nodes': [
            {'reference': [':1.3', '/native/close'], 'name': '关闭', 'role': 'push button', 'windowReference': native},
            {'reference': [':1.3', '/native/off/1'], 'name': '关闭', 'role': 'radio button', 'windowReference': native},
            {'reference': [':1.3', '/native/off/2'], 'name': '关闭', 'role': 'radio button', 'windowReference': native},
            {'reference': [':1.3', '/main/close'], 'name': '关闭', 'role': 'push button', 'windowReference': main},
        ]}
        probe = object.__new__(Probe)
        probe.active_frame_reference = native
        probe.snapshot = lambda: snapshot
        self.assertEqual(len(probe.nodes('关闭')), 3)
        self.assertEqual([node['reference'][1] for node in probe.nodes('关闭', 'push button')], ['/native/close'])

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
