"""Browser temp selection fixes nested scratch paths without escaping approved roots."""
from __future__ import annotations

import importlib
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from tools.tests.browser_tmp_fixtures import deep_below_scratch


class BrowserTempRoot(unittest.TestCase):
    def selector(self):
        try:
            module = importlib.import_module('tools.browser_tmp')
        except ModuleNotFoundError:
            module = None
        self.assertIsNotNone(module, 'Central Chromium temporary-root selection is missing')
        return module

    def test_nested_tmp_uses_a_short_owned_scratch_ancestor(self) -> None:
        module = self.selector()
        with TemporaryDirectory() as tmp:
            base = Path(tmp)
            anchor = base / 'scratch'
            anchor.mkdir()
            deep = anchor / ('nested-' + 'x' * 100)
            deep.mkdir()
            # Budget pinned to this anchor: no outer `scratch` of the runner's TMPDIR decides.
            with patch('tools.browser_tmp.MAX_SOCKET_BYTES', self.fitting_budget(module, anchor)):
                self.assertFalse(module.fits_temp_root(deep))
                chosen = module.select_temp_root(deep)
                self.assertTrue(module.fits_temp_root(chosen))
            self.assertIn(chosen, deep.resolve().parents)
            self.assertEqual(chosen.name, 'scratch')
            self.assertEqual(chosen.stat().st_uid, os.getuid())
            self.assertTrue(deep.exists(), 'must not delete the supplied directory')

    def fitting_budget(self, module, root: Path) -> int:
        """Socket budget at which `root` exactly fits as an allocation parent."""
        return len(os.fsencode(str(root.resolve()) + module.OWNED_TEMP_SUFFIX
                               + module.CHROMIUM_SOCKET_SUFFIX))

    def test_symlinked_scratch_component_is_an_approved_anchor(self) -> None:
        # A configured `.../scratch` may be a symlink (e.g. moved to another disk);
        # resolving must not erase the approval its name carries.
        module = self.selector()
        with TemporaryDirectory() as tmp:
            work = Path(tmp)
            target = work / 'moved'
            target.mkdir(mode=0o700)
            (work / 'profile').mkdir()
            link = work / 'profile' / 'scratch'
            link.symlink_to(target, target_is_directory=True)
            deep = link / ('nested-' + 'x' * 100)
            deep.mkdir()
            with patch('tools.browser_tmp.MAX_SOCKET_BYTES', self.fitting_budget(module, target)):
                self.assertFalse(module.fits_temp_root(deep))
                self.assertEqual(module.select_temp_root(deep), target.resolve())
            self.assertTrue(deep.exists(), 'must not delete the supplied directory')

    def test_symlinked_scratch_to_a_shared_directory_is_refused(self) -> None:
        module = self.selector()
        with TemporaryDirectory() as tmp:
            work = Path(tmp)
            target = work / 'shared'
            target.mkdir()
            target.chmod(0o777)
            (work / 'profile').mkdir()
            link = work / 'profile' / 'scratch'
            link.symlink_to(target, target_is_directory=True)
            deep = link / ('nested-' + 'x' * 100)
            deep.mkdir()
            with patch('tools.browser_tmp.MAX_SOCKET_BYTES', self.fitting_budget(module, target)):
                # Outer approved ancestors of the test's own TMPDIR may still qualify.
                self.assertNotIn(target.resolve(), module.scratch_candidates(deep))
                if module.scratch_candidates(deep) == []:
                    with self.assertRaisesRegex(ValueError, 'short approved'):
                        module.select_temp_root(deep)

    def test_explicit_root_and_missing_approved_fallback_fail_closed(self) -> None:
        module = self.selector()
        with TemporaryDirectory() as tmp:
            base = Path(tmp)
            anchor = base / 'scratch'
            anchor.mkdir()
            deep = anchor / ('nested-' + 'x' * 100)
            deep.mkdir()
            with patch('tools.browser_tmp.scratch_candidates', return_value=[]):
                with self.assertRaisesRegex(ValueError, 'short approved'):
                    module.select_temp_root(deep)
            with self.assertRaisesRegex(ValueError, 'socket'):
                module.select_temp_root(deep, explicit=deep)
            with self.assertRaises((OSError, ValueError)):
                module.select_temp_root(deep, explicit=base / 'missing')

    def test_every_playwright_entry_installs_the_same_guard(self) -> None:
        root = Path(__file__).resolve().parents[2]
        configs = sorted((root / 'apps/knx-web').glob('playwright*.config.ts'))
        self.assertGreater(len(configs), 0)
        missing = [p.name for p in configs if 'import "./browserTmp";' not in p.read_text()]
        self.assertEqual(missing, [], 'Standalone and inherited configs need the central temp guard')
    def test_actual_config_loader_repairs_deep_tmp_and_cleans_only_its_directory(self) -> None:
        root = Path(__file__).resolve().parents[2]
        app = root / 'apps/knx-web'
        with deep_below_scratch() as (work, deep):
            marker = deep / 'keep.txt'
            marker.write_text('retained')
            metadata = work / 'environment.json'
            config = work / 'probe.config.ts'
            # A scratch-located CJS config must also load the app's ESM/TS guard.
            config.write_text(
                f'import base from {json.dumps(str(app / "playwright.config.ts"))};\n'
                'import {writeFileSync,statSync} from "node:fs";\n'
                f'writeFileSync({json.dumps(str(metadata))}, JSON.stringify({{path:process.env.TMPDIR,'
                'uid:statSync(process.env.TMPDIR!).uid,mode:statSync(process.env.TMPDIR!).mode & 511}));\n'
                f'export default {{...base,testDir:{json.dumps(str(app / "e2e"))}}};\n')
            env = {k: v for k, v in os.environ.items() if not k.startswith('KNX')}
            env['TMPDIR'] = str(deep)
            done = subprocess.run(['node', 'node_modules/@playwright/test/cli.js', 'test',
                                   f'--config={config}', '--list', '--reporter=json',
                                   f'--output={work / "artifacts"}'], cwd=app, env=env,
                                  capture_output=True, text=True, timeout=30)
            self.assertEqual(done.returncode, 0, (done.stdout + done.stderr)[-2000:])
            self.assertGreater(len(json.loads(done.stdout)['suites']), 0)
            observed = json.loads(metadata.read_text())
            module = self.selector()
            self.assertLessEqual(len(os.fsencode(observed['path'] + module.CHROMIUM_SOCKET_SUFFIX)),
                                 module.MAX_SOCKET_BYTES)
            self.assertEqual(Path(observed['path']).parent, module.select_temp_root(deep))
            self.assertEqual(observed['uid'], os.getuid())
            self.assertEqual(observed['mode'], 0o700)
            self.assertFalse(Path(observed['path']).exists(), 'owned guard directory leaked')
            self.assertEqual(marker.read_text(), 'retained')
    def test_parent_owned_directory_is_reused_without_an_extra_path_component(self) -> None:
        module = self.selector()
        self.assertTrue(hasattr(module, 'validate_owned_directory'), 'Parent-owned temp reuse is missing')
        short = module.select_temp_root(Path(os.environ.get('TMPDIR', '/tmp')))
        with TemporaryDirectory(prefix='w', dir=short) as tmp:
            owned = Path(tmp)
            self.assertEqual(module.validate_owned_directory(owned), owned.resolve())
            result = subprocess.run([sys.executable, module.__file__, '--base', str(owned),
                                     '--reuse-owned'], check=True, capture_output=True, text=True)
            self.assertEqual(json.loads(result.stdout), str(owned.resolve()))
            owned.chmod(0o755)
            with self.assertRaisesRegex(ValueError, 'private'):
                module.validate_owned_directory(owned)


if __name__ == '__main__':
    unittest.main()
