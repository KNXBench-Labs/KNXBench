"""Regress the recurring leaf-root refusal and unguarded standalone launches."""
from __future__ import annotations

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import unittest

from tools import browser_tmp
from tools.tests.browser_tmp_fixtures import deep_below_scratch, short_workspace

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / 'apps/knx-web'
STATUS_FIELDS = ('State', 'PPid', 'NSpgid', 'NSsid', 'Threads', 'SigPnd', 'ShdPnd', 'SigBlk', 'SigIgn',
                 'SigCgt')


def process_state(pid: int) -> str:
    """Kernel view of one process: run state, wait channel and signal masks."""
    proc = Path('/proc') / str(pid)
    try:
        status = dict(line.split(':\t', 1) for line in (proc / 'status').read_text().splitlines()
                      if ':\t' in line)
        wchan = (proc / 'wchan').read_text()
    except OSError as error:
        return f'{pid}: gone ({error.strerror})'
    fields = ' '.join(f'{key}={status.get(key, "?").strip()}' for key in STATUS_FIELDS)
    return f'{pid}: {fields} wchan={wchan or "-"}'


def diagnose_stuck_wrapper(wrapper: subprocess.Popen, directory: Path) -> str:
    """Describe a wrapper that did not finish after SIGTERM, then end it.

    Records the wrapper and every process carrying its temp directory (the
    child group), aborts them so PYTHONFAULTHANDLER prints their Python
    stacks, and returns all of it as the failure message.
    """
    marker = f'KNXBENCH_BROWSER_TMP_DIR={directory}'.encode()
    group = []
    for entry in Path('/proc').iterdir():
        try:
            if entry.name.isdigit() and marker in (entry / 'environ').read_bytes().split(b'\0'):
                group.append(int(entry.name))
        except OSError:
            continue
    states = [f'load={os.getloadavg()}', f'wrapper exit={wrapper.poll()}',
              process_state(wrapper.pid), *(process_state(pid) for pid in group)]
    # One at a time, so the stacks do not interleave on the shared stderr pipe.
    for pid in (wrapper.pid, *group):
        try:
            os.kill(pid, signal.SIGABRT)
        except ProcessLookupError:
            continue
        for _ in range(100):
            if process_state(pid).split(' ', 2)[1] in ('gone', 'State=Z', 'State=X'):
                break
            time.sleep(0.03)
    try:
        _, stderr = wrapper.communicate(timeout=10)
    except subprocess.TimeoutExpired:
        for pid in (wrapper.pid, *group):
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        _, stderr = wrapper.communicate(timeout=10)
    stacks = [line for line in stderr.splitlines() if not line.lstrip().startswith('Binary file')]
    return ('wrapper pipes still open 10 s after SIGTERM\n' + '\n'.join(states) + '\n'
            + '\n'.join(stacks)[-6000:])


class BrowserTempLaunch(unittest.TestCase):
    def clean_environment(self) -> dict[str, str]:
        return {key: value for key, value in os.environ.items()
                if not key.startswith(('KNX', 'PLAYWRIGHT_'))}

    def workspace(self):
        return short_workspace()

    def leaf_root(self, scratch: Path) -> Path:
        # A root that fits Chromium itself, but cannot fit another /wXXXXXXXX.
        size = browser_tmp.MAX_SOCKET_BYTES - len(os.fsencode(browser_tmp.CHROMIUM_SOCKET_SUFFIX))
        leaf = scratch / ('x' * (size - len(os.fsencode(scratch)) - 1))
        leaf.mkdir(mode=0o700)
        self.assertFalse(browser_tmp.fits_temp_root(leaf))
        browser_tmp.validate_owned_directory(leaf)
        return leaf

    def test_direct_config_reuses_private_explicit_leaf_and_preserves_it(self) -> None:
        with self.workspace() as tmp:
            work = Path(tmp)
            leaf = self.leaf_root(work)
            marker = leaf / 'keep'
            marker.write_text('caller owns this')
            metadata = work / 'observed.json'
            config = work / 'probe.config.ts'
            config.write_text(
                f'import base from {json.dumps(str(APP / "playwright.config.ts"))};\n'
                'import {writeFileSync} from "node:fs";\n'
                f'writeFileSync({json.dumps(str(metadata))}, JSON.stringify(process.env.TMPDIR));\n'
                f'export default {{...base,testDir:{json.dumps(str(APP / "e2e"))}}};\n')
            env = {**self.clean_environment(), 'TMPDIR': str(leaf),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(leaf)}
            result = subprocess.run(
                ['node', 'node_modules/@playwright/test/cli.js', 'test',
                 f'--config={config}', '--list', '--reporter=json'],
                cwd=APP, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, (result.stdout + result.stderr)[-3000:])
            self.assertEqual(json.loads(metadata.read_text()), str(leaf))
            self.assertEqual(marker.read_text(), 'caller owns this')

    def test_python_context_reuses_leaf_without_deleting_caller_data(self) -> None:
        self.assertTrue(hasattr(browser_tmp, 'browser_directory'), 'Shared allocator is missing')
        with self.workspace() as tmp:
            leaf = self.leaf_root(Path(tmp))
            marker = leaf / 'keep'
            marker.write_text('retained')
            with browser_tmp.browser_directory(leaf, explicit=leaf) as directory:
                self.assertEqual(directory, leaf)
            self.assertEqual(marker.read_text(), 'retained')
            leaf.chmod(0o755)
            with self.assertRaisesRegex(ValueError, 'private'):
                with browser_tmp.browser_directory(leaf, explicit=leaf):
                    self.fail('must not reuse a non-private leaf')

    def test_standalone_preload_repairs_deep_tmp_before_loading_playwright(self) -> None:
        with deep_below_scratch() as (work, deep):
            marker = deep / 'keep'
            marker.write_text('retained')
            env = {**self.clean_environment(), 'TMPDIR': str(deep)}
            program = (
                "const fs=require('node:fs');"
                "const path=process.env.TMPDIR;"
                "if(Buffer.byteLength(path + '/org.chromium.Chromium.XXXXXX/SingletonSocket')>107)"
                "throw Error('unprotected Chromium temp path');"
                "console.log(JSON.stringify({path,mode:fs.statSync(path).mode&511}));")
            result = subprocess.run(
                ['node', '--require', str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=work, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            observed = json.loads(result.stdout)
            self.assertEqual(Path(observed['path']).parent, browser_tmp.select_temp_root(deep))
            self.assertEqual(observed['mode'], 0o700)
            self.assertFalse(Path(observed['path']).exists())
            self.assertEqual(marker.read_text(), 'retained')

    def test_cli_wraps_a_standalone_command_and_cleans_after_failure(self) -> None:
        with deep_below_scratch() as (work, deep):
            program = 'import os,sys;print(os.environ["TMPDIR"]);sys.exit(23)'
            result = subprocess.run(
                [sys.executable, str(ROOT / 'tools/browser_tmp.py'), '--base', str(deep),
                 '--run', sys.executable, '-c', program],
                env=self.clean_environment(), capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 23, result.stderr)
            directory = Path(result.stdout.strip())
            self.assertEqual(directory.parent, browser_tmp.select_temp_root(deep))
            self.assertFalse(directory.exists())
            self.assertTrue(deep.exists())

    def test_real_chromium_launches_at_the_validated_leaf_boundary(self) -> None:
        with deep_below_scratch() as (work, deep):
            leaf = self.leaf_root(work)
            env = {**self.clean_environment(), 'TMPDIR': str(leaf),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(leaf)}
            program = (
                "const fs=require('node:fs');const {chromium}=require('playwright');"
                "(async()=>{const profile=process.env.TMPDIR+'/p';"
                "const context=await chromium.launchPersistentContext(profile,"
                "{executablePath:'/usr/bin/chromium',headless:true});"
                "try{const page=await context.newPage();"
                "await page.goto('data:text/html,actual-browser-proof');"
                "if(await page.locator('body').textContent()!=='actual-browser-proof')throw Error('body');"
                "console.log(JSON.stringify({directory:process.env.TMPDIR,"
                "socket:fs.readlinkSync(profile+'/SingletonSocket')}));"
                "}finally{await context.close();}})().catch(e=>{console.error(e);process.exit(1)});")
            result = subprocess.run(
                ['unshare', '--user', '--map-root-user', '--net', 'node', '--require',
                 str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=APP, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr[-4000:])
            observed = json.loads(result.stdout)
            self.assertEqual(observed['directory'], str(leaf))
            self.assertLessEqual(len(os.fsencode(observed['socket'])), browser_tmp.MAX_SOCKET_BYTES)
            self.assertTrue(leaf.exists(), 'caller-owned leaf must survive browser shutdown')
            result = subprocess.run(
                ['unshare', '--user', '--map-root-user', '--net', 'node', '--require',
                 str(ROOT / 'tools/browser_tmp.cjs'), '-e', program],
                cwd=APP, env={**self.clean_environment(), 'TMPDIR': str(deep)},
                capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr[-4000:])
            observed = json.loads(result.stdout)
            self.assertLessEqual(len(os.fsencode(observed['socket'])), browser_tmp.MAX_SOCKET_BYTES)
            self.assertFalse(Path(observed['directory']).exists(), 'owned browser temp leaked')
            self.assertTrue(deep.exists(), 'deep caller directory must survive')

    def test_nested_node_process_reuses_parent_directory_without_owning_cleanup(self) -> None:
        with deep_below_scratch() as (work, deep):
            guard = str(ROOT / 'tools/browser_tmp.cjs')
            child = "console.log(JSON.stringify(process.env.TMPDIR))"
            program = (
                "const {execFileSync}=require('node:child_process');const fs=require('node:fs');"
                "const directory=process.env.TMPDIR;"
                f"const nested=JSON.parse(execFileSync(process.execPath,['--require',{json.dumps(guard)},"
                f"'-e',{json.dumps(child)}],{{encoding:'utf8'}}));"
                "if(nested!==directory||!fs.existsSync(directory))throw Error('parent ownership lost');"
                "console.log(JSON.stringify(directory));")
            result = subprocess.run(['node', '--require', guard, '-e', program],
                                    cwd=work, env={**self.clean_environment(), 'TMPDIR': str(deep)},
                                    capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(Path(json.loads(result.stdout)).exists())
            self.assertTrue(deep.exists())

    def test_cli_sigterm_waits_for_child_shutdown_before_cleanup(self) -> None:
        import select
        with self.workspace() as tmp:
            work = Path(tmp)
            # The child blocks SIGTERM before it announces itself and then takes
            # it synchronously. A handler followed by time.sleep lost the signal
            # when it landed while print() returned: CPython 3.14 runs no pending
            # handler after a keyword call, so the sleep blocked for a minute.
            program = ('import os,signal,sys;'
                       'signal.pthread_sigmask(signal.SIG_BLOCK,{signal.SIGTERM});'
                       'print(os.environ["TMPDIR"],flush=True);'
                       'sys.exit(0 if signal.sigtimedwait({signal.SIGTERM},60) else 1)')
            # PYTHONFAULTHANDLER only acts on fatal signals: on a timeout the
            # diagnosis aborts both processes and gets their Python stacks.
            child = subprocess.Popen(
                [sys.executable, str(ROOT / 'tools/browser_tmp.py'), '--base', str(work),
                 '--run', sys.executable, '-c', program],
                env={**self.clean_environment(), 'PYTHONFAULTHANDLER': '1'},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                self.assertTrue(select.select([child.stdout], [], [], 10)[0], 'child did not start')
                directory = Path(child.stdout.readline().strip())
                self.assertTrue(directory.is_dir())
                child.send_signal(signal.SIGTERM)
                try:
                    stdout, stderr = child.communicate(timeout=10)
                except subprocess.TimeoutExpired:
                    self.fail(diagnose_stuck_wrapper(child, directory))
                self.assertEqual(child.returncode, 0, stderr)
                self.assertFalse(directory.exists())
                self.assertTrue(work.exists())
            finally:
                if child.poll() is None:
                    child.kill()
                    child.communicate(timeout=10)

    def test_multibyte_and_whitespace_paths_use_byte_budget_and_json(self) -> None:
        with self.workspace() as tmp:
            work = Path(tmp)
            # Its character count fits while its UTF-8 byte count does not.
            remaining = 107 - len(os.fsencode(work)) - 1 - len(browser_tmp.CHROMIUM_SOCKET_SUFFIX)
            bad = work / ('ä' * (remaining // 2 + 1))
            bad.mkdir(mode=0o700)
            self.assertLessEqual(len(str(bad) + browser_tmp.CHROMIUM_SOCKET_SUFFIX), 107)
            self.assertGreater(len(os.fsencode(str(bad) + browser_tmp.CHROMIUM_SOCKET_SUFFIX)), 107)
            with self.assertRaisesRegex(ValueError, 'socket'):
                browser_tmp.allocate_browser_directory(bad, explicit=bad)
            spaced = work / '\n'
            spaced.mkdir(mode=0o700)
            env = {**self.clean_environment(), 'TMPDIR': str(spaced),
                   'KNXBENCH_BROWSER_TMP_ROOT': str(spaced)}
            result = subprocess.run(
                ['node', '--require', str(ROOT / 'tools/browser_tmp.cjs'), '-e',
                 'console.log(JSON.stringify(process.env.TMPDIR))'],
                cwd=work, env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            directory = Path(json.loads(result.stdout))
            if browser_tmp.fits_temp_root(spaced):
                self.assertEqual(directory.parent, spaced)
                self.assertFalse(directory.exists())
            else:
                self.assertEqual(directory, spaced)
            self.assertTrue(spaced.exists())

    def test_standalone_repository_launchers_load_guard_before_playwright(self) -> None:
        for relative in (
            'apps/knx-web/scripts/workbench-browser-proof.mjs',
            'design/verify-crt-interactions.mjs',
            'design/verify-crt-reference.mjs',
            'story/tests/browser/check_story.mjs',
        ):
            with self.subTest(path=relative):
                source = (ROOT / relative).read_text()
                self.assertTrue('browser_tmp.cjs' in source, 'standalone launch bypasses temp guard')
                self.assertLess(source.index('browser_tmp.cjs'), source.index('chromium.launch')
                                if 'chromium.launch' in source else source.index('require("playwright")'))


if __name__ == '__main__':
    unittest.main()
