"""Reject features that are declared complete without both client surfaces and tests.

This is a structural parity gate, not proof of UX equivalence. CI runs the real
Rust and Flutter tests separately.
"""
import json
import sys
from pathlib import Path

REQUIRED = ('core', 'egui', 'flutter', 'rust_test', 'flutter_test')


def check(root: Path) -> list[str]:
    errors = []
    manifest = json.loads((root / 'feature-parity.json').read_text(encoding='utf-8'))
    features = manifest.get('features', [])
    if not features:
        return ['feature-parity.json: no registered features']
    seen = set()
    for feature in features:
        name = feature.get('id', '<unnamed>')
        if name in seen:
            errors.append(f'{name}: duplicate id')
        seen.add(name)
        for surface in REQUIRED:
            entry = feature.get(surface)
            if not isinstance(entry, dict) or not entry.get('path') or not entry.get('symbol'):
                errors.append(f'{name}: {surface} path and symbol required')
                continue
            relative = Path(entry['path'])
            path = (root / relative).resolve()
            if not path.is_relative_to(root.resolve()) or not path.is_file():
                errors.append(f'{name}: {surface} invalid path {relative}')
            elif entry['symbol'] not in path.read_text(encoding='utf-8'):
                errors.append(f'{name}: {surface} missing {entry["symbol"]} in {relative}')
    return errors


if __name__ == '__main__':
    problems = check(Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[1])
    if problems:
        print('\n'.join(problems), file=sys.stderr)
        sys.exit(1)
    print('Feature parity contract: OK')
