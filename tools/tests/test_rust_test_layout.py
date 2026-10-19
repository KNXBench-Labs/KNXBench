"""Rust integration tests stay bundled: one binary per crate, no file left out."""
from __future__ import annotations

from pathlib import Path
import re
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
# Test files that keep a process of their own, with the reason (ADR-0144).
STANDALONE = {
    'crates/knx-productdb/tests/large_member_memory.rs':
        'asserts the peak RSS (VmHWM) of one install; other tests in the process would inflate it',
}
MOD = re.compile(r'^\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_][a-z0-9_]*)\s*;', re.M)


def members(root: Path) -> list[Path]:
    manifests = [*root.glob('crates/*/Cargo.toml'), *root.glob('apps/*/Cargo.toml'),
                 *root.glob('apps/*/src-tauri/Cargo.toml'), root / 'xtask/Cargo.toml']
    return sorted(m.parent for m in manifests if m.is_file())


def layout_problems(root: Path) -> list[str]:
    problems = []
    for crate in members(root):
        tests = crate / 'tests'
        if not tests.is_dir():
            continue
        loose = sorted(p for p in tests.glob('*.rs')
                       if p.relative_to(root).as_posix() not in STANDALONE)
        bundles = sorted(p.parent for p in tests.glob('*/main.rs'))
        name = crate.relative_to(root).as_posix()
        if len(bundles) > 1:
            problems.append(f'{name}: {len(bundles)} bundled test binaries; keep one (tests/it)')
        if bundles and loose:
            problems.append(f'{name}: move {", ".join(p.name for p in loose)} into tests/it '
                            f'and declare it in tests/it/main.rs')
        if not bundles and len(loose) > 1:
            problems.append(f'{name}: {len(loose)} test binaries; bundle them in tests/it (ADR-0144)')
        for bundle in bundles:
            declared = set(MOD.findall((bundle / 'main.rs').read_text(encoding='utf-8')))
            present = {p.stem for p in bundle.glob('*.rs') if p.name != 'main.rs'}
            present |= {p.parent.name for p in bundle.glob('*/mod.rs')}
            for missing in sorted(present - declared):
                problems.append(f'{bundle.relative_to(root).as_posix()}/{missing}: never compiled; '
                                f'add `mod {missing};` to main.rs')
    for path in sorted(STANDALONE):
        if not (root / path).is_file():
            problems.append(f'{path}: listed as standalone but missing; update the allowlist')
    return problems


class RustTestLayout(unittest.TestCase):
    def test_the_repository_bundles_every_crate(self) -> None:
        self.assertEqual(layout_problems(ROOT), [])

    def test_an_undeclared_file_and_a_new_loose_binary_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            crate = root / 'crates/knx-x'
            (crate / 'tests/it/helper').mkdir(parents=True)
            (crate / 'Cargo.toml').write_text('[package]\nname = "knx-x"\n')
            (crate / 'tests/it/main.rs').write_text('mod one;\n#[allow(dead_code)]\nmod helper;\n')
            (crate / 'tests/it/one.rs').write_text('')
            (crate / 'tests/it/helper/mod.rs').write_text('')
            self.assertEqual([p for p in layout_problems(root) if 'knx-x' in p], [])
            (crate / 'tests/it/two.rs').write_text('')
            (crate / 'tests/loose.rs').write_text('')
            problems = [p for p in layout_problems(root) if 'knx-x' in p]
            self.assertEqual(len(problems), 2, problems)
            self.assertIn('two: never compiled', problems[1])

    def test_a_crate_may_keep_one_unbundled_test_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            crate = root / 'crates/knx-y'
            (crate / 'tests').mkdir(parents=True)
            (crate / 'Cargo.toml').write_text('[package]\nname = "knx-y"\n')
            (crate / 'tests/only.rs').write_text('')
            self.assertEqual([p for p in layout_problems(root) if 'knx-y' in p], [])
            (crate / 'tests/second.rs').write_text('')
            self.assertEqual(len([p for p in layout_problems(root) if 'knx-y' in p]), 1)


if __name__ == '__main__':
    unittest.main()
