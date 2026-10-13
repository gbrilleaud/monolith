import json
import tempfile
import unittest
from pathlib import Path

from scripts.check_feature_parity import check


class FeatureParityTests(unittest.TestCase):
    def test_rejects_missing_flutter_surface(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'src').mkdir()
            (root / 'flutter').mkdir()
            (root / 'tests').mkdir()
            (root / 'src/core.rs').write_text('pub fn controller_profile() {}')
            (root / 'src/ui.rs').write_text('controller_profile()')
            (root / 'flutter/ui.dart').write_text('// no control yet')
            (root / 'tests/core.rs').write_text('fn test_controller_profile() {}')
            (root / 'tests/ui.dart').write_text('testWidgets("controller profile", () {})')
            manifest = {'features': [{'id': 'controller_profiles', 'core': {'path': 'src/core.rs', 'symbol': 'controller_profile'}, 'egui': {'path': 'src/ui.rs', 'symbol': 'controller_profile'}, 'flutter': {'path': 'flutter/ui.dart', 'symbol': 'controllerProfiles'}, 'rust_test': {'path': 'tests/core.rs', 'symbol': 'test_controller_profile'}, 'flutter_test': {'path': 'tests/ui.dart', 'symbol': 'controller profile'}}]}
            (root / 'feature-parity.json').write_text(json.dumps(manifest))
            self.assertIn('flutter', '\n'.join(check(root)))
            (root / 'flutter/ui.dart').write_text('controllerProfiles()')
            self.assertEqual(check(root), [])

    def test_rejects_missing_test_and_path_traversal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'feature-parity.json').write_text(json.dumps({'features': [{'id': 'bad', 'core': {'path': '../outside', 'symbol': 'x'}}]}))
            errors = check(root)
            self.assertTrue(any('egui' in error for error in errors))
            self.assertTrue(any('path' in error for error in errors))


if __name__ == '__main__':
    unittest.main()
