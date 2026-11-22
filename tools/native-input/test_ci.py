#!/usr/bin/env python3
"""Offline CI refusal checks, not native-window observations."""

import copy
import json
from pathlib import Path
import runpy
import tempfile
import unittest

from ci import PROGRAM_PATHS, validate_run, verify_identity, write_json
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
                        'binarySha256': self.identity['programs']['verdant']['sha256'],
                        'workerSha256': self.identity['programs']['verdant-worker']['sha256']}
        self.report = {'format': 'verdant.native-input-observation.v1', 'backend': 'x11', 'ok': True,
                       'skipSetValue': False, 'cases': [{'name': name, 'status': 'passed'} for name in CASES],
                       'binary': self.identity['programs']['verdant'],
                       'worker': self.identity['programs']['verdant-worker']}
        self.save()

    def save(self):
        write_json(self.run / 'harness.json', self.harness)
        write_json(self.evidence / 'result.json', self.report)

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


if __name__ == '__main__':
    unittest.main()
