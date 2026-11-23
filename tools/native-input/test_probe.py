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


class TextFocusObserved(Exception):
    """Stop after the actual selector and focus predicate accept a native node."""


class CollectionEntrySelected(Exception):
    """Stop before clicking the observed library entry in the native window."""


class ProbeHelpers(unittest.TestCase):
    def test_collection_entry_uses_the_native_push_button_in_its_own_window(self):
        main, other = [':1.3', '/frame/0'], [':1.3', '/frame/1']
        selected = {'reference': [':1.3', '/library'], 'name': '预设库',
                    'role': 'push button', 'roleNumber': 43, 'windowReference': main}
        snapshot = {'nodes': [selected,
            {**selected, 'reference': [':1.3', '/other/library'], 'windowReference': other},
            {**selected, 'reference': [':1.3', '/generic'], 'role': 'button'},
        ]}
        probe = object.__new__(Probe)
        probe.active_frame_reference = main
        probe.snapshot = lambda: snapshot
        probe.args = SimpleNamespace(timeout=0)
        probe.process = None
        probe.persisted = lambda: {'revision': 2}
        probe.click = Mock(side_effect=CollectionEntrySelected)
        with self.assertRaises(CollectionEntrySelected):
            probe.collection_scroll_cancel()
        probe.click.assert_called_once_with(selected)

    def collection_probe(self):
        current, other = [':1.3', '/frame/1'], [':1.3', '/frame/0']
        viewport = {'reference': [':1.3', '/collection'], 'name': '匹配的预设',
                    'role': 'list box', 'windowReference': current, 'windowExtents': [168, 148, 716, 340]}
        first = {'reference': [':1.3', '/option/0'], 'name': 'Deep Sub', 'role': 'list item',
                 'windowReference': current, 'states': ['selected'], 'windowExtents': [168, 148, 716, 24]}
        hidden = {**first, 'reference': [':1.3', '/option/19'], 'name': 'Soft Pad',
                  'states': [], 'windowExtents': [168, 604, 716, 24]}
        snapshot = {'nodes': [viewport, first, hidden,
            {**viewport, 'reference': [':1.3', '/other/collection'], 'windowReference': other},
            {**viewport, 'reference': [':1.3', '/legacy'], 'name': '预设列表', 'role': 'document frame'},
            {**viewport, 'reference': [':1.3', '/wrong-role'], 'role': 'document frame'},
            {**first, 'reference': [':1.3', '/other/option'], 'windowReference': other},
        ]}
        probe = object.__new__(Probe)
        probe.active_frame_reference = current
        probe.snapshot = lambda: snapshot
        probe.args = SimpleNamespace(timeout=0)
        probe.process = None
        return probe, snapshot, viewport

    def test_collection_observation_uses_the_compiled_listbox_in_its_current_window(self):
        # Both retained native failures already expose this exact ListBox;
        # the obsolete document-frame selector never reached its real rows.
        probe, _, viewport = self.collection_probe()
        observed = probe.collection_observation()
        self.assertEqual(observed['viewportReference'], viewport['reference'])
        self.assertEqual(observed['viewport'], [168, 148, 716, 340])
        self.assertEqual([row['reference'] for row in observed['options']],
                         [[':1.3', '/option/0'], [':1.3', '/option/19']])
        self.assertEqual([row['visible'] for row in observed['options']], [True, False])
        self.assertEqual([row['selected'] for row in observed['options']], [True, False])

    def test_collection_observation_rejects_missing_wrong_or_ambiguous_listbox(self):
        for mutation in ('missing', 'role', 'name', 'window', 'duplicate'):
            with self.subTest(mutation=mutation):
                probe, snapshot, viewport = self.collection_probe()
                if mutation == 'missing': snapshot['nodes'].remove(viewport)
                elif mutation == 'role': viewport['role'] = 'document frame'
                elif mutation == 'name': viewport['name'] = '预设列表'
                elif mutation == 'window': viewport['windowReference'] = [':1.3', '/other-frame']
                else: snapshot['nodes'].append({**viewport, 'reference': [':1.3', '/duplicate']})
                message = 'Ambiguous accessible target' if mutation == 'duplicate' else 'Timed out waiting'
                with self.assertRaisesRegex(RuntimeError, message):
                    probe.collection_observation()

    def test_text_journey_selects_the_native_push_button_after_tab(self):
        frame = [':1.3', '/frame/0']
        field = {'reference': [':1.3', '/input'], 'name': '输入文字…', 'role': 'entry',
                 'states': ['focusable'], 'windowReference': frame}
        cancel = {'reference': [':1.3', '/cancel'], 'name': '取消', 'role': 'push button',
                  'roleNumber': 43, 'states': ['focusable', 'focused'], 'windowReference': frame}
        decoy = {'reference': [':1.3', '/generic'], 'name': '取消', 'role': 'button',
                 'states': ['focusable'], 'windowReference': frame}
        snapshot = {'nodes': [field, cancel, decoy]}
        probe = object.__new__(Probe)
        probe.active_frame_reference = frame
        probe.snapshot = lambda: snapshot
        probe.args = SimpleNamespace(timeout=0)
        probe.process = None
        probe.persisted = lambda: {'revision': 0, 'panPermille': 0}
        probe.rename_prompt = lambda: field
        probe.type_unicode = Mock()
        probe.text = Mock()
        probe.key = Mock()
        probe.log_input = Mock()
        probe.observer = SimpleNamespace(
            set_text_selection=Mock(return_value=True),
            read=lambda reference: next(n for n in snapshot['nodes'] if tuple(n['reference']) == reference))

        def record(label, **evidence):
            if label == 'text-focus-forward':
                self.assertEqual(evidence['control']['focused'], cancel)
                raise TextFocusObserved()

        probe.record = Mock(side_effect=record)
        with self.assertRaises(TextFocusObserved):
            probe.text_selection_focus()
        self.assertEqual(probe.key.call_args.args, ('Tab',))

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


# These fixtures test the evidence gate. They are not native-window results.
import copy
import hashlib
import json
from controls import (CONTROL_CASES, SEARCH_MISSING, SEARCH_NAME, TEXT_ACCEPTED, TEXT_ASSISTIVE,
                      TEXT_INITIAL, intersects, pan_after_displacement, validate_control_trace)


def synthetic_control_observations():
    baseline = {'revision': 0, 'project_id': 'offline-fixture', 'tracks': [
        {'name': 'original', 'notes': [], 'instrument': {'params': {'gain': -8.0}}}]}

    def project(revision, name='original', pan=None):
        result = copy.deepcopy(baseline)
        result['revision'] = revision
        result['tracks'][0]['name'] = name
        if pan is not None:
            result['tracks'][0]['pan_permille'] = pan
        return result

    def collection(selected, offset):
        # The production synth ships exactly twenty presets. PageUp from End
        # clamps to the first row; a made-up twenty-first row is not required.
        return {'viewport': [0, 0, 100, 48], 'options': [
            {'name': f'Preset {index:02}', 'reference': [':1.2', f'/option/{index}'],
             'selected': index == selected, 'bounds': [0, index * 12 - offset, 100, 12],
             'visible': 0 <= index * 12 - offset < 48} for index in range(20)]}

    entry = {'name': '输入文字…', 'role': 'entry', 'states': ['focused'], 'reference': [':1.2', '/input']}
    cancel = {'name': '取消', 'role': 'push button', 'states': ['focused'], 'reference': [':1.2', '/cancel']}
    search = {'name': SEARCH_NAME, 'role': 'entry', 'states': ['focused'], 'reference': [':1.2', '/search']}
    sequence = [
        ('preflight', 0, {}, None, None),
        ('text-unicode', 0, {'text': {'text': TEXT_INITIAL}, 'imeComposition': False}, None, None),
        ('text-astral-selection', 0, {'text': {'text': TEXT_INITIAL, 'selections': [[1, 2]]}}, None, None),
        ('text-assistive-selection', 0, {'text': {'text': TEXT_INITIAL, 'selections': [[2, 4]]},
         'action': {'method': 'Text.SetSelection', 'arguments': [0, 2, 4], 'accepted': True}}, None, None),
        ('text-assistive-edited', 0, {'text': {'text': TEXT_ASSISTIVE}}, None, None),
        ('text-edited', 0, {'text': {'text': TEXT_ACCEPTED}}, None, None),
        ('text-focus-forward', 0, {'focused': cancel}, None, None),
        ('text-focus-back', 0, {'focused': entry, 'text': {'text': TEXT_ACCEPTED}}, None, None),
        ('text-committed', 1, {}, TEXT_ACCEPTED, None),
        ('text-single-undo', 2, {}, None, None),
        ('text-canceled', 2, {}, None, None),
        ('collection-open', 2, collection(0, 0), None, None),
        ('collection-end', 2, collection(19, 192), None, None),
        ('collection-page-up', 2, collection(0, 0), None, None),
        ('collection-home', 2, collection(0, 0), None, None),
        ('collection-wheel', 2, collection(0, 48), None, None),
        ('collection-search-empty', 2, {'text': {'text': SEARCH_MISSING}, 'options': []}, None, None),
        ('collection-search-navigation', 2, {**collection(1, 0), 'focused': search, 'text': {'text': ''}}, None, None),
        ('collection-canceled', 2, {}, None, None),
        ('mixer-preview', 2, {'pan': {'value': {'CurrentValue': 33.3}}, 'width': 100, 'displacement': 14}, None, None),
        ('mixer-committed', 3, {'pan': {'value': {'CurrentValue': 42.9}}, 'width': 100, 'displacement': 18}, None, 429),
        ('mixer-single-undo', 4, {}, None, None),
        ('mixer-canceled-while-held', 4, {'pan': {'value': {'CurrentValue': 0}}}, None, None),
        ('mixer-canceled', 4, {}, None, None),
    ]
    observations = []
    for label, revision, control, name, pan in sequence:
        document = project(revision, name or 'original', pan)
        case = next((case for case in CONTROL_CASES if label.startswith(case.split('-')[0] + '-')), None)
        control = copy.deepcopy(control)
        frame = [':1.2', '/frame']
        nodes = [{'name': 'fixture', 'role': 'frame', 'reference': frame, 'windowReference': frame}]
        if 'text' in control:
            field = copy.deepcopy(search if label.startswith('collection-') else entry)
            field['interfaces'] = ['org.a11y.atspi.Text']
            control['text']['reference'] = field['reference']
            text = control['text']
            selected = text.setdefault('selections', [])
            text['caret'] = selected[-1][1] if selected else (
                2 if label in ('text-edited', 'text-focus-back') else len(text['text']))
            sample = {key: copy.deepcopy(text[key]) for key in ('text', 'caret', 'selections')}
            text['samples'] = [copy.deepcopy(sample), copy.deepcopy(sample)]
            nodes.append(field)
        if 'focused' in control:
            focus = copy.deepcopy(control['focused'])
            if not any(node['reference'] == focus['reference'] for node in nodes):
                nodes.append(focus)
        if 'viewport' in control:
            control['viewportReference'] = [':1.2', '/viewport']
            nodes.append({'reference': control['viewportReference'], 'name': '匹配的预设',
                          'role': 'list box', 'windowExtents': copy.deepcopy(control['viewport'])})
            nodes.extend({'reference': item['reference'], 'name': item['name'], 'role': 'list item',
                          'states': ['selected'] if item['selected'] else [], 'windowExtents': copy.deepcopy(item['bounds'])}
                         for item in control['options'])
        if 'pan' in control:
            control['pan'].update({'reference': [':1.2', '/pan'], 'name': '轨道声像', 'role': 'slider',
                                   'interfaces': ['org.a11y.atspi.Value'], 'windowExtents': [0, 0, 100, 24]})
            nodes.append(copy.deepcopy(control['pan']))
        for node in nodes:
            node['windowReference'] = frame
        detail = {'label': label, 'case': case, 'control': control,
                  'pointerHeld': label in ('mixer-preview', 'mixer-canceled-while-held'),
                  'savedProject': {'sha256': hashlib.sha256(json.dumps(document, sort_keys=True).encode()).hexdigest()},
                  'windowReference': frame, 'tree': {'nodes': nodes}}
        observations.append((detail, document))
    return observations, project(4)


class ControlEvidenceRefusals(unittest.TestCase):
    def setUp(self):
        self.observations, self.final = synthetic_control_observations()

    def detail(self, label):
        return next(detail for detail, _ in self.observations if detail['label'] == label)

    def document(self, label):
        return next(document for detail, document in self.observations if detail['label'] == label)

    def test_twenty_factory_rows_and_complete_durable_story_are_admitted(self):
        validate_control_trace(self.observations, self.final)

    def test_unicode_is_not_ime_and_astral_selection_uses_character_offsets(self):
        self.detail('text-unicode')['control']['imeComposition'] = True
        with self.assertRaises(RuntimeError):
            validate_control_trace(self.observations, self.final)
        self.setUp()
        self.detail('text-astral-selection')['control']['text']['selections'] = [[1, 3]]
        with self.assertRaises(RuntimeError):
            validate_control_trace(self.observations, self.final)

    def test_assistive_selection_needs_changed_range_and_subsequent_editor_input(self):
        self.detail('text-assistive-selection')['control']['text']['selections'] = [[1, 2]]
        with self.assertRaises(RuntimeError):
            validate_control_trace(self.observations, self.final)
        self.setUp()
        self.detail('text-assistive-edited')['control']['text']['text'] = '音乐AB'
        with self.assertRaises(RuntimeError):
            validate_control_trace(self.observations, self.final)

    def test_text_commit_cannot_hide_an_unrelated_domain_change(self):
        self.document('text-committed')['tracks'][0]['instrument']['params']['gain'] = -2
        with self.assertRaisesRegex(RuntimeError, 'another domain'):
            validate_control_trace(self.observations, self.final)

    def test_focus_summary_cannot_replace_actual_tree_state(self):
        self.detail('text-focus-forward')['tree']['nodes'] = []
        with self.assertRaisesRegex(RuntimeError, 'actual tree'):
            validate_control_trace(self.observations, self.final)

    def test_cancel_requires_the_platform_push_button_role_and_actual_focus(self):
        # A real AT-SPI Button is role 43, named "push button". The original
        # X11 trace had already focused it when the generic "button" query
        # timed out. Keep role and physical focus checks independently strict.
        self.detail('text-focus-forward')['control']['focused']['role'] = 'button'
        next(node for node in self.detail('text-focus-forward')['tree']['nodes']
             if node.get('reference') == self.detail('text-focus-forward')['control']['focused']['reference'])['role'] = 'button'
        with self.assertRaisesRegex(RuntimeError, 'wrong physical focus target'):
            validate_control_trace(self.observations, self.final)
        self.setUp()
        self.detail('text-focus-forward')['control']['focused']['states'] = []
        with self.assertRaisesRegex(RuntimeError, 'focus summary differs from the actual tree'):
            validate_control_trace(self.observations, self.final)

    def test_scroll_requires_changed_visible_rows_and_consistent_native_geometry(self):
        self.detail('collection-wheel')['control'] = copy.deepcopy(self.detail('collection-home')['control'])
        self.detail('collection-wheel')['tree'] = copy.deepcopy(self.detail('collection-home')['tree'])
        with self.assertRaisesRegex(RuntimeError, 'visible rows'):
            validate_control_trace(self.observations, self.final)
        self.setUp()
        self.detail('collection-wheel')['control']['options'][0]['visible'] = True
        with self.assertRaisesRegex(RuntimeError, 'platform geometry'):
            validate_control_trace(self.observations, self.final)

    def test_end_requires_last_row_selected_and_visible(self):
        self.detail('collection-end')['control']['options'][-1]['selected'] = False
        last = self.detail('collection-end')['control']['options'][-1]['reference']
        next(node for node in self.detail('collection-end')['tree']['nodes']
             if node.get('reference') == last)['states'] = []
        with self.assertRaisesRegex(RuntimeError, 'selection/reveal'):
            validate_control_trace(self.observations, self.final)

    def test_terminal_sample_cannot_commit_last_observed_preview(self):
        self.document('mixer-committed')['tracks'][0]['pan_permille'] = 333
        with self.assertRaisesRegex(RuntimeError, 'final sample'):
            validate_control_trace(self.observations, self.final)

    def test_cancel_requires_held_pointer_and_unchanged_saved_bytes(self):
        self.detail('mixer-canceled-while-held')['pointerHeld'] = False
        with self.assertRaisesRegex(RuntimeError, 'before the native'):
            validate_control_trace(self.observations, self.final)
        self.setUp()
        self.detail('text-canceled')['savedProject']['sha256'] = 'changed'
        with self.assertRaisesRegex(RuntimeError, 'saved bytes'):
            validate_control_trace(self.observations, self.final)

    def test_missing_or_duplicate_observation_cannot_satisfy_passed_flags(self):
        with self.assertRaisesRegex(RuntimeError, 'Incomplete'):
            validate_control_trace(self.observations[:-1], self.final)
        with self.assertRaisesRegex(RuntimeError, 'Duplicate'):
            validate_control_trace(self.observations + [self.observations[-1]], self.final)

    def test_nonfinite_or_empty_geometry_cannot_authorize_a_value(self):
        for width in (0, 16, float('nan'), float('inf')):
            with self.assertRaises(RuntimeError):
                pan_after_displacement(0, 5, width)
        with self.assertRaises(RuntimeError):
            intersects([0, 0, float('nan'), 10], [0, 0, 10, 10])
        self.assertFalse(intersects([0, 0, 0, 10], [0, 0, 10, 10]))

    def test_text_reference_interface_and_window_must_exist_in_raw_tree(self):
        for mutation in ('interface', 'window', 'reference'):
            with self.subTest(mutation=mutation):
                self.setUp()
                detail = self.detail('text-unicode')
                field = next(node for node in detail['tree']['nodes'] if node.get('role') == 'entry')
                if mutation == 'interface': field['interfaces'] = []
                elif mutation == 'window': field['windowReference'] = [':1.2', '/another-window']
                else: field['reference'] = [':1.2', '/missing-field']
                with self.assertRaises(RuntimeError):
                    validate_control_trace(self.observations, self.final)

    def test_pan_summary_cannot_contradict_raw_value_or_bounds(self):
        for mutation in ('value', 'bounds'):
            with self.subTest(mutation=mutation):
                self.setUp()
                node = next(node for node in self.detail('mixer-committed')['tree']['nodes']
                            if node.get('role') == 'slider')
                if mutation == 'value': node['value']['CurrentValue'] = 0
                else: node['windowExtents'][2] = 99
                with self.assertRaisesRegex(RuntimeError, 'actual tree'):
                    validate_control_trace(self.observations, self.final)

    def test_collection_viewport_requires_exact_raw_role_label_and_current_window(self):
        for mutation in ('role', 'name', 'window', 'duplicate-reference'):
            with self.subTest(mutation=mutation):
                self.setUp()
                detail = self.detail('collection-open')
                viewport = next(node for node in detail['tree']['nodes']
                                if node.get('reference') == detail['control']['viewportReference'])
                if mutation == 'role': viewport['role'] = 'document frame'
                elif mutation == 'name': viewport['name'] = '预设列表'
                elif mutation == 'window': viewport['windowReference'] = [':1.2', '/another-window']
                else: detail['tree']['nodes'].append(copy.deepcopy(viewport))
                with self.assertRaises(RuntimeError):
                    validate_control_trace(self.observations, self.final)

    def test_collection_summary_cannot_replace_raw_selection_bounds_or_focus(self):
        for mutation in ('selection', 'viewport', 'inventory', 'search-focus'):
            with self.subTest(mutation=mutation):
                self.setUp()
                detail = self.detail('collection-search-navigation' if mutation == 'search-focus' else 'collection-end')
                nodes = detail['tree']['nodes']
                if mutation == 'selection':
                    next(node for node in nodes if node.get('states') == ['selected'])['states'] = []
                elif mutation == 'viewport':
                    next(node for node in nodes if node.get('reference') == detail['control']['viewportReference'])['windowExtents'] = [0, 0, 5, 5]
                elif mutation == 'inventory':
                    nodes.remove(next(node for node in nodes if node.get('role') == 'list item'))
                else:
                    next(node for node in nodes if node.get('role') == 'entry')['states'] = []
                with self.assertRaisesRegex(RuntimeError, 'actual'):
                    validate_control_trace(self.observations, self.final)

    def test_text_summary_requires_two_equal_unmodified_raw_final_samples(self):
        for mutation in ('missing', 'changing', 'invalid', 'summary'):
            with self.subTest(mutation=mutation):
                self.setUp()
                value = self.detail('text-unicode')['control']['text']
                if mutation == 'missing': value['samples'] = []
                elif mutation == 'changing': value['samples'][-1]['caret'] = 0
                elif mutation == 'invalid':
                    value['caret'] = 99
                    for sample in value['samples']: sample['caret'] = 99
                else: value['caret'] = 0
                with self.assertRaisesRegex(RuntimeError, 'platform Text'):
                    validate_control_trace(self.observations, self.final)

    def test_stable_search_prefix_with_no_rows_cannot_replace_the_complete_query(self):
        value = self.detail('collection-search-empty')['control']['text']
        value['text'] = SEARCH_MISSING[:-1]
        value['caret'] = len(value['text'])
        sample = {key: copy.deepcopy(value[key]) for key in ('text', 'caret', 'selections')}
        value['samples'] = [copy.deepcopy(sample), copy.deepcopy(sample)]
        with self.assertRaisesRegex(RuntimeError, 'Search did not remove'):
            validate_control_trace(self.observations, self.final)


if __name__ == '__main__':
    unittest.main()
