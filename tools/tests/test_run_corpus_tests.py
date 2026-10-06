"""The corpus-test runner finds every corpus test and never passes on nothing."""
from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from tools.run_corpus_tests import corpus_tests, main, scrubbed_environment

MARK = '#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]'


def crate(root: Path, name: str, files: dict[str, str], lib: bool = True, bin_: bool = False) -> None:
    base = root / 'crates' / name
    (base / 'src').mkdir(parents=True)
    (base / 'Cargo.toml').write_text(f'[package]\nname = "{name}"\n')
    if lib:
        (base / 'src' / 'lib.rs').write_text('')
    if bin_:
        (base / 'src' / 'main.rs').write_text('')
    for rel, text in files.items():
        path = base / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)


class CorpusSelection(unittest.TestCase):
    def test_finds_corpus_tests_by_target_and_skips_other_ignores(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate(root, 'knx-a', {
                'src/model.rs': f'#[test]\n{MARK}\nfn reads_the_reference() {{}}\n'
                                '#[test]\n#[ignore = "needs hardware"]\nfn talks_to_a_device() {}\n',
                'tests/golden.rs': f'#[tokio::test]\n{MARK}\nasync fn golden_counts() {{}}\n',
            }, bin_=True)
            targets, problems = corpus_tests(root)
            self.assertEqual(problems, [])
            self.assertEqual(
                [(t.package, t.flags, t.names) for t in targets],
                [('knx-a', ('--lib', '--bins'), ['reads_the_reference']),
                 ('knx-a', ('--test', 'golden'), ['golden_counts'])],
            )

    def test_an_attribute_without_a_test_is_a_problem(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate(root, 'knx-b', {'src/x.rs': f'{MARK}\nconst NOT_A_TEST: u8 = 0;\n'})
            _, problems = corpus_tests(root)
            self.assertEqual(len(problems), 1)

    def test_a_missing_corpus_is_not_a_pass(self) -> None:
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate(root, 'knx-c', {'src/x.rs': f'{MARK}\nfn t() {{}}\n'})
            self.assertEqual(main(['--root', str(root)]), 2)

    def test_the_environment_drops_knx_variables_and_isolates_xdg(self) -> None:
        import os
        os.environ['KNX_GATEWAY'] = '192.0.2.1'
        try:
            env = scrubbed_environment('/tmp/isolated')
        finally:
            del os.environ['KNX_GATEWAY']
        self.assertNotIn('KNX_GATEWAY', env)
        self.assertEqual(env['XDG_DATA_HOME'], '/tmp/isolated')


if __name__ == '__main__':
    unittest.main()
