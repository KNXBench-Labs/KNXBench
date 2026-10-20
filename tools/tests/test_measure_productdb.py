"""Synthetic-only controls for the private ProductDB measurement observer."""
from __future__ import annotations

import copy
import hashlib
import importlib
import importlib.util
import json
import os
import signal
import shutil
import sqlite3
import subprocess
import sys
from unittest.mock import patch
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


def measurement(test):
    test.assertIsNotNone(importlib.util.find_spec('tools.measure_productdb'),
                         'WP04.1 measurement API is missing')
    return importlib.import_module('tools.measure_productdb')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def refusal_manifest(root):
    (root / 'refused.knxprod').write_bytes(b'synthetic refusal input')
    return {'format': 1, 'inputRoot': str(root), 'inputs': [{
        'id': 'refused', 'path': 'refused.knxprod',
        'sha256': sha(b'synthetic refusal input'), 'kind': 'package',
        'retainedPackages': []}], 'cases': [{'id': 'rollback', 'steps': [{
            'op': 'install', 'input': 'refused', 'profile': 'standard',
            'expectExit': 1, 'diagnosticSuffix': 'named refusal', 'preserveAll': True}]}]}


class ManifestTests(unittest.TestCase):
    def test_manifest_is_strict_nonempty_and_references_only_admitted_inputs(self):
        api = measurement(self)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            good = refusal_manifest(root)
            path = root / 'manifest.json'
            path.write_text(json.dumps(good))
            self.assertEqual(api.load_manifest(path), good)
            bads = []
            for key, value in [('format', True), ('cases', []), ('extra', 1)]:
                bads.append({**good, key: value})
            for key, value in [('id', 'bad/id'), ('sha256', 'A' * 64), ('path', '../escape.knxprod'),
                               ('kind', 'unknown'), ('retainedPackages', None)]:
                bad = copy.deepcopy(good)
                bad['inputs'][0][key] = value
                bads.append(bad)
            bad = copy.deepcopy(good)
            bad['inputs'] *= 2
            bads.append(bad)
            for key, value in [('input', 'missing'), ('profile', 'huge'), ('op', 'shell'),
                               ('expectExit', True), ('diagnosticSuffix', None),
                               ('preserveAll', False)]:
                bad = copy.deepcopy(good)
                bad['cases'][0]['steps'][0][key] = value
                bads.append(bad)
            for bad in bads:
                path.write_text(json.dumps(bad))
                with self.subTest(bad=bad), self.assertRaises(ValueError):
                    api.load_manifest(path)
            path.write_text('{"format":1,"format":1}')
            with self.assertRaises(ValueError):
                api.load_manifest(path)
            nested = root / 'admitted'
            nested.mkdir()
            escaped = refusal_manifest(nested)
            (nested / 'escape.knxprod').symlink_to(root / 'refused.knxprod')
            escaped['inputs'][0]['path'] = 'escape.knxprod'
            path.write_text(json.dumps(escaped))
            with self.assertRaisesRegex(ValueError,'escapes'):
                api.load_manifest(path)


class OperationTests(unittest.TestCase):
    def test_owned_environment_exact_argv_and_direct_child_outcomes(self):
        api = measurement(self)
        self.assertTrue(hasattr(api, 'owned_environment'), 'isolated operation API is missing')
        from tools.web_gate_support import MeasuredRun
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            env = api.owned_environment(root / 'isolation')
            names = ('HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME',
                     'XDG_STATE_HOME', 'XDG_RUNTIME_DIR', 'TMPDIR')
            run = MeasuredRun(root / 'processes')
            child = run.step('env', [sys.executable, '-c',
                'import json,os;print(json.dumps({k:os.environ[k] for k in '+repr(names)+'}))'], root, env)
            actual = json.loads((run.output / 'env.log').read_text())
            self.assertEqual(actual, {name: env[name] for name in names})
            self.assertTrue(all(Path(value).is_relative_to(root) for value in actual.values()))
            self.assertEqual(Path(env['XDG_RUNTIME_DIR']).stat().st_mode & 0o777, 0o700)
            self.assertGreater(child['peakRssKiB'], 0)
            source, db = root / 'same.knxprod', root / 'owned.sqlite'
            step = {'op': 'install', 'profile': 'standard', 'expectExit': 1,
                    'diagnosticSuffix': 'named refusal', 'preserveAll': True}
            command = api.install_argv(Path('/bin/knx'), source, db, step['profile'])
            self.assertEqual(command, ['/bin/knx', 'products', 'ingest', str(source),
                                       '--product-db', str(db)])
            self.assertEqual(command, api.install_argv(Path('/bin/knx'), source, db, 'standard'))
            self.assertNotEqual(command[3], api.install_argv(Path('/bin/knx'), root / 'alias.knxprod', db, 'large')[3])
            stage = run.step('refused', [sys.executable, '-c',
                'print("private prefix: named refusal");raise SystemExit(1)'], root, env)
            self.assertTrue(api.outcome(stage, run.output / 'refused.log', step)['expected'])
            self.assertFalse(api.outcome(stage, run.output / 'refused.log',
                                        {**step, 'diagnosticSuffix': 'different refusal'})['expected'])
            killed = run.step('signal', [sys.executable, '-c',
                'import os,signal;os.kill(os.getpid(),signal.SIGTERM)'], root, env)
            self.assertEqual(killed['exit'], -signal.SIGTERM)
            self.assertGreater(killed['peakRssKiB'], 0)
            self.assertFalse(api.outcome(killed, run.output / 'signal.log', step)['expected'])
            self.assertFalse(run.finish('same', 'same'))
            self.assertFalse(json.loads((run.output / 'receipt.json').read_text())['success'])

    def test_resolver_summary_keeps_null_unavailable_and_nonempty_nonexecuting_distinct(self):
        api = measurement(self)
        with TemporaryDirectory() as tmp:
            log = Path(tmp) / 'resolver.log'
            step = {'op':'resolve','expectStatus':'expanded','minimumSteps':2,'requiredIssueCodes':[]}
            result = {'status':'expanded','executable':False,'steps':2,'unplaced':0,
                      'issueCodes':[],'omittedIssueCount':0}
            envelope = {'format':1,'operation':'resolve-ap1','schema':28,'result':result}
            log.write_text(json.dumps(envelope))
            self.assertTrue(api.outcome({'exit':0},log,step)['expected'])
            for value in (None, {**result,'executable':True}, {**result,'steps':1},
                          {**result,'status':'unavailable'}, {**result,'steps':True}, {**result,'extra':1}):
                log.write_text(json.dumps({**envelope,'result':value}))
                self.assertFalse(api.outcome({'exit':0},log,step)['expected'])
            unavailable = {**result,'status':'unavailable','steps':0,'issueCodes':['source-byte-limit']}
            log.write_text(json.dumps({**envelope,'result':unavailable}))
            outcome = api.outcome({'exit':0},log,{'op':'resolve','expectStatus':'unavailable',
                'minimumSteps':0,'requiredIssueCodes':['source-byte-limit']})
            self.assertTrue(outcome['expected'])
            self.assertEqual(outcome['resolveResult'],unavailable)
            self.assertFalse(api.outcome({'exit':1},log,step)['expected'])

    def test_observer_identity_closes_its_connection_before_return(self):
        api = measurement(self)
        original = sqlite3.connect
        opened = []
        class Tracked(sqlite3.Connection):
            closed = False
            def close(self):
                self.closed = True
                return super().close()
        def connect(*args,**kwargs):
            connection = original(*args,**kwargs,factory=Tracked)
            opened.append(connection)
            return connection
        with patch.object(sqlite3,'connect',connect):
            api.observer_identity()
        try:
            self.assertTrue(opened[0].closed, 'observer capability connection must be closed before child execution')
        finally:
            for connection in opened:
                connection.close()

    def test_prior_large_child_does_not_contaminate_small_child_peak(self):
        api = measurement(self)
        self.assertTrue(hasattr(api, 'owned_environment'), 'isolated operation API is missing')
        from tools.web_gate_support import MeasuredRun
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            run = MeasuredRun(root / 'processes')
            env = api.owned_environment(root / 'home')
            large = run.step('large', [sys.executable, '-c', 'data=bytearray(96*1024*1024)'], root, env)
            small = run.step('small', [sys.executable, '-c', 'pass'], root, env)
            self.assertEqual((large['exit'], small['exit']), (0, 0))
            self.assertNotEqual(large['pid'], small['pid'])
            self.assertGreater(large['peakRssKiB'], small['peakRssKiB'] + 48 * 1024)

    def test_paths_cannot_overlap_or_escape_before_creation(self):
        api = measurement(self)
        self.assertTrue(hasattr(api, 'validate_output'), 'owned output validation API is missing')
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = root / 'inputs'
            inputs.mkdir()
            checkout = root / 'checkout'
            checkout.mkdir()
            (root / 'link').symlink_to(inputs, target_is_directory=True)
            (root / 'dangling').symlink_to(root / 'new-target')
            for output in (inputs / 'out', root, root / 'link' / 'out', root / 'dangling',
                           checkout / 'out', Path('relative')):
                with self.subTest(output=output), self.assertRaises(ValueError):
                    api.validate_output(output, inputs, checkout)
            self.assertEqual(api.validate_output(root / 'new', inputs, checkout), (root / 'new').resolve())
            self.assertFalse((root / 'new').exists())


class StudyTests(unittest.TestCase):
    def setUp(self):
        # The complete web gate already owns the real checkout's workspace lease.
        # Synthetic studies still exercise a REAL lease, but in their own repo.
        directory = TemporaryDirectory(prefix='measurement-checkout-')
        self.addCleanup(directory.cleanup)
        checkout = Path(directory.name)
        subprocess.run(['git', 'init', '--quiet', str(checkout)],
                       check=True, capture_output=True)
        root = patch.object(measurement(self), 'ROOT', checkout)
        root.start()
        self.addCleanup(root.stop)

    def test_synthetic_study_fixture_has_private_checkout_for_its_lease(self):
        api = measurement(self)
        source_checkout = Path(__file__).resolve().parents[2]
        self.assertNotEqual(api.ROOT.resolve(), source_checkout,
                            'synthetic studies must not reacquire the outer gate checkout lease')
        self.assertTrue((api.ROOT / '.git').is_dir())

    def observer_inputs(self, root):
        from argparse import Namespace
        inputs = root / 'inputs'
        inputs.mkdir()
        manifest = inputs / 'manifest.json'
        manifest.write_text(json.dumps(refusal_manifest(inputs)))
        # These executables are observer controls, NOT a production installer,
        # release build or ProductDB schema/compatibility fixture.
        knx = root / 'synthetic-refusal'
        knx.write_text('#!' + sys.executable + '\nprint("private prefix: named refusal")\nraise SystemExit(1)\n')
        probe = root / 'synthetic-setup'
        probe.write_text('#!' + sys.executable + '\n' +
            'import json,sqlite3,sys\n' +
            'p=sys.argv[sys.argv.index("--product-db")+1]\n' +
            'db=sqlite3.connect(p)\n' +
            'db.executescript("PRAGMA user_version=28;CREATE TABLE package (sha256 TEXT PRIMARY KEY,size INT,bytes BLOB);'
            'CREATE TABLE source_file (sha256 TEXT PRIMARY KEY,source_path TEXT,manufacturer_id TEXT,len INT,bytes BLOB);'
            'CREATE TABLE package_member (package_sha256 TEXT,ordinal INT,path TEXT,role TEXT,source_sha256 TEXT,size INT);")\n' +
            'db.close()\nprint(json.dumps({"format":1,"operation":"init","schema":28,"sqliteVersion":sqlite3.sqlite_version}))\n')
        for path in (knx, probe):
            path.chmod(0o700)
        from tools.tests.test_productdb_process_observer import synthetic_helper
        supervisor = synthetic_helper(root)
        build = root / 'synthetic-build-control.json'
        build.write_text(json.dumps({'format': 1, 'success': True, 'sourceCommit': 'a' * 40,
                                    'sourceFingerprint': 'b' * 64, 'profile': 'release',
                                    'executables': {name: {'sha256': sha(path.read_bytes())}
                                                    for name, path in [('knx', knx), ('probe', probe), ('supervisor', supervisor)]}}))
        return Namespace(knx=knx, probe=probe, supervisor=supervisor, build_receipt=build, manifest=manifest,
                         output=root / 'attempt', repetitions=1, lease_timeout=1.0)

    def test_study_credits_target_pid_and_binds_native_supervisor_identity(self):
        api = measurement(self)
        from tools.tests.test_productdb_process_observer import synthetic_helper
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            args.supervisor = synthetic_helper(Path(tmp))
            with patch.object(api, 'source_identity', return_value=('a'*40, 'b'*64, False)):
                result = api.run_study(args)
            self.assertTrue(result['accepted'])
            raw = json.loads((args.output / 'processes/receipt.json').read_text())['stages'][0]
            target = result['observations'][0]['process']
            self.assertNotEqual(target['pid'], raw['pid'])
            self.assertEqual(target['supervisorPid'], raw['pid'])
            self.assertEqual(set(result['executables']), {'knx', 'probe', 'supervisor'})
            self.assertEqual(result['observer']['rssMethod'], 'native-supervisor-wait4-target')
            self.assertTrue((args.output / 'processes/r0-c0-s0.target.json').is_file())

    def test_missing_native_proof_still_reads_database_after_completed_wrapper(self):
        api = measurement(self)
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            args.supervisor.write_text('#!' + sys.executable + '\nraise SystemExit(0)\n')
            build = json.loads(args.build_receipt.read_text())
            build['executables']['supervisor']['sha256'] = sha(args.supervisor.read_bytes())
            args.build_receipt.write_text(json.dumps(build))
            with patch.object(api, 'source_identity', return_value=('a'*40, 'b'*64, False)):
                result = api.run_study(args)
            self.assertFalse(result['complete'])
            self.assertFalse(result['accepted'])
            observation = result['observations'][0]
            self.assertIsNotNone(observation['after'])
            self.assertTrue(observation['integrity']['preservationExact'])
            self.assertIsNone(observation['process']['peakRssKiB'])
            self.assertTrue(observation['process']['observerError'])
            raw = json.loads((args.output / 'processes/receipt.json').read_text())
            self.assertEqual(raw['stages'][0]['exit'], 0)
            self.assertTrue(result['errors'])

    def test_expected_refusal_has_full_after_accounting_and_unchanged_raw_failure(self):
        api = measurement(self)
        self.assertTrue(hasattr(api, 'run_study'), 'source-bound study orchestrator API is missing')
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            args.repetitions = 2
            with patch.object(api, 'source_identity', return_value=('a' * 40, 'b' * 64, False)):
                result = api.run_study(args)
            self.assertTrue(result['complete'])
            self.assertTrue(result['accepted'])
            observation = result['observations'][0]
            self.assertEqual(observation['process']['exit'], 1)
            self.assertGreater(observation['process']['peakRssKiB'], 0)
            self.assertTrue(all(observation['integrity'].values()))
            self.assertIsNotNone(observation['after'])
            raw = json.loads((args.output / 'processes/receipt.json').read_text())
            self.assertFalse(raw['success'])
            self.assertEqual(raw['stages'][0]['exit'], 1)
            self.assertEqual(len(raw['stages']), 2)
            self.assertEqual(raw['stages'][0]['command'][7],raw['stages'][1]['command'][7])
            self.assertNotEqual(raw['stages'][0]['command'][9],raw['stages'][1]['command'][9])
            self.assertEqual((args.output.stat().st_mode & 0o777), 0o700)
            self.assertEqual(json.loads((args.output / 'receipt.json').read_text()), result)
            if evidence := os.environ.get('WP041_TEST_EVIDENCE'):
                shutil.copytree(args.output, Path(evidence) / 'expected-refusal-observer-control')
                binaries = Path(evidence) / 'synthetic-control-executables'
                binaries.mkdir(mode=0o700)
                for name, path in [('knx',args.knx),('probe',args.probe)]:
                    shutil.copyfile(path,binaries / name)

    def test_after_accounting_error_and_identity_drift_prevent_acceptance(self):
        api = measurement(self)
        self.assertTrue(hasattr(api, 'run_study'), 'source-bound study orchestrator API is missing')
        from tools import productdb_measurement_accounting as accounting
        for mode in ('after-read', 'binary', 'input', 'source', 'spawn'):
            with self.subTest(mode=mode), TemporaryDirectory() as tmp:
                args = self.observer_inputs(Path(tmp))
                real_snapshot = accounting.snapshot
                calls = []
                def snapshot(path):
                    calls.append(path)
                    if len(calls) == 2:
                        if mode == 'after-read':
                            raise ValueError('synthetic after-read failure')
                        if mode == 'binary':
                            args.knx.write_text('changed after execution')
                        if mode == 'input':
                            (args.manifest.parent / 'refused.knxprod').write_bytes(b'changed')
                    return real_snapshot(path)
                identity_calls = []
                def identity(_):
                    identity_calls.append(1)
                    return ('a' * 40, ('c' if mode == 'source' and len(identity_calls) > 1 else 'b') * 64, False)
                from tools.web_gate_support import MeasuredRun
                real_step = MeasuredRun.step
                def step(run, *pos, **kw):
                    if mode == 'spawn':
                        raise FileNotFoundError('synthetic spawn failure')
                    return real_step(run, *pos, **kw)
                with patch.object(api, 'source_identity', side_effect=identity), \
                     patch.object(accounting, 'snapshot', side_effect=snapshot), \
                     patch.object(MeasuredRun, 'step', step):
                    result = api.run_study(args)
                self.assertFalse(result['accepted'])
                if mode in ('after-read', 'spawn'):
                    self.assertFalse(result['complete'])
                self.assertTrue(result['errors'])
                self.assertFalse(json.loads((args.output / 'receipt.json').read_text())['accepted'])

    def test_artifact_binding_failure_cannot_leave_an_accepted_receipt(self):
        api = measurement(self)
        original = api.save_json
        def fail_binding(path, value):
            if path.name == 'receipt-binding.json':
                raise OSError('synthetic artifact binding write failure')
            return original(path,value)
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            with patch.object(api,'source_identity',return_value=('a'*40,'b'*64,False)), \
                 patch.object(api,'save_json',side_effect=fail_binding):
                try:
                    result = api.run_study(args)
                except OSError:
                    result = json.loads((args.output / 'receipt.json').read_text())
            self.assertFalse(result['accepted'], 'failed required artifact binding must invalidate receipt acceptance')
            self.assertFalse(json.loads((args.output / 'receipt.json').read_text())['accepted'])

    def test_lease_timeout_records_real_queue_wait_without_running_a_child(self):
        api = measurement(self)
        from contextlib import contextmanager
        import time
        from tools import run_package_gate
        @contextmanager
        def timeout(_root,_run,_timeout):
            time.sleep(0.01)
            raise TimeoutError('synthetic lease wait expired')
            yield
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            with patch.object(api,'source_identity',return_value=('a'*40,'b'*64,False)), \
                 patch.object(run_package_gate,'workspace_lease',timeout):
                result = api.run_study(args)
            self.assertFalse(result['complete'])
            self.assertFalse(result['accepted'])
            self.assertGreaterEqual(result['leaseWaitSeconds'],0.01,
                                    'expired lease wait must not be reported as zero')
            self.assertFalse(result['observations'])
            self.assertFalse((args.output / 'r0-c0').exists())

    def test_binary_change_after_build_validation_cannot_become_start_baseline(self):
        api = measurement(self)
        original_observer = api.observer_identity
        for name in ('knx', 'probe', 'supervisor'):
            with self.subTest(binary=name), TemporaryDirectory() as tmp:
                args = self.observer_inputs(Path(tmp))
                binary = getattr(args, name)
                authenticated = sha(binary.read_bytes())
                def replace_after_validation():
                    binary.write_bytes(binary.read_bytes() + b'\n# changed after build validation\n')
                    return original_observer()
                with patch.object(api, 'source_identity', return_value=('a'*40, 'b'*64, False)), \
                     patch.object(api, 'observer_identity', side_effect=replace_after_validation):
                    with self.assertRaisesRegex(ValueError, 'build binary mismatch'):
                        api.run_study(args)
                self.assertNotEqual(sha(binary.read_bytes()), authenticated)
                self.assertFalse(args.output.exists(), 'mismatched binary must refuse before execution')

    def test_strict_build_receipt_and_unsupported_observer_refuse_before_output(self):
        api = measurement(self)
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            identity = ('a'*40,'b'*64,False)
            binaries = {'knx':args.knx,'probe':args.probe,'supervisor':args.supervisor}
            original = json.loads(args.build_receipt.read_text())
            self.assertEqual(api.load_build_receipt(args.build_receipt,identity,binaries),original)
            for field, value in [('format',True),('success',False),('profile','debug'),
                                  ('sourceCommit','c'*40),('sourceFingerprint','d'*64),('extra',1)]:
                args.build_receipt.write_text(json.dumps({**original,field:value}))
                with self.subTest(field=field),self.assertRaises(ValueError):
                    api.load_build_receipt(args.build_receipt,identity,binaries)
            args.build_receipt.write_text(json.dumps(original))
            with self.assertRaises(ValueError):
                api.load_build_receipt(args.build_receipt,('a'*40,'b'*64,True),binaries)
            with patch.object(sys,'platform','darwin'),self.assertRaisesRegex(ValueError,'unsupported'):
                api.observer_identity()
            self.assertFalse(args.output.exists())

    def test_missing_peak_or_reap_failure_keeps_study_incomplete(self):
        api = measurement(self)
        from tools.web_gate_support import MeasuredRun
        original = MeasuredRun.step
        for mode in ('missing-peak', 'reap'):
            with self.subTest(mode=mode), TemporaryDirectory() as tmp:
                args = self.observer_inputs(Path(tmp))
                def missing_peak(run, *pos, **kwargs):
                    result = dict(original(run,*pos,**kwargs))
                    result.pop('peakRssKiB')
                    return result
                with patch.object(api, 'source_identity', return_value=('a'*40,'b'*64,False)):
                    if mode == 'missing-peak':
                        with patch.object(MeasuredRun,'step',missing_peak):
                            result = api.run_study(args)
                    else:
                        with patch('os.wait4',side_effect=OSError('synthetic reap failure')):
                            result = api.run_study(args)
                self.assertFalse(result['accepted'])
                self.assertFalse(result['complete'], 'missing per-child observation is incomplete, not zero RSS')
                self.assertTrue(result['errors'])

    def test_accounting_after_zero_exit_fails_closed_and_retains_both_binary_end_hashes(self):
        api = measurement(self)
        from tools import productdb_measurement_accounting as accounting
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            args.knx.write_text('#!' + sys.executable + '\nraise SystemExit(0)\n')
            build = json.loads(args.build_receipt.read_text())
            build['executables']['knx']['sha256'] = sha(args.knx.read_bytes())
            args.build_receipt.write_text(json.dumps(build))
            original = accounting.snapshot
            calls = []
            def after_error(path):
                calls.append(1)
                if len(calls) > 1:
                    raise ValueError('zero-exit after-accounting error')
                return original(path)
            with patch.object(api, 'source_identity', return_value=('a'*40,'b'*64,False)), \
                 patch.object(accounting, 'snapshot', side_effect=after_error):
                result = api.run_study(args)
            self.assertFalse(result['complete'])
            self.assertFalse(result['accepted'])
            self.assertEqual(result['observations'][0]['process']['exit'],0)
            self.assertIsNone(result['observations'][0]['after'])
            self.assertTrue(all(v['sha256End'] for v in result['executables'].values()))

    def test_identity_refusal_still_records_each_end_hash_and_binds_retained_artifacts(self):
        api = measurement(self)
        with TemporaryDirectory() as tmp:
            args = self.observer_inputs(Path(tmp))
            with patch.object(api, 'source_identity', side_effect=[('a'*40,'b'*64,False),
                                                                  ('a'*40,'c'*64,False)]):
                result = api.run_study(args)
            self.assertFalse(result['accepted'])
            self.assertTrue(all(v['sha256End'] for v in result['executables'].values()),
                            'source drift must not skip binary end observations')
            self.assertTrue((args.output / 'receipt-binding.json').is_file(),
                            'manifest/log/stream artifacts must be bound to the study receipt')
            binding = json.loads((args.output / 'receipt-binding.json').read_text())
            self.assertEqual(binding['receiptSha256'],sha((args.output / 'receipt.json').read_bytes()))
            self.assertEqual(binding['artifacts']['manifest.original.json'],sha(args.manifest.read_bytes()))

    def test_console_does_not_disclose_input_path_hash_or_raw_error(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            private = root / 'private-secret-input.json'
            private.write_text('invalid private raw diagnostic')
            result = subprocess.run([sys.executable, '-m', 'tools.measure_productdb', 'run',
                '--knx', '/missing-knx', '--probe', '/missing-probe', '--supervisor', '/missing-supervisor',
                '--build-receipt', str(private),
                '--manifest', str(private), '--output', str(root / 'out'), '--repetitions', '1'],
                capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            output = result.stdout + result.stderr
            self.assertNotIn(str(private), output)
            self.assertNotIn('invalid private raw diagnostic', output)
            self.assertNotIn('Traceback', output)
            self.assertFalse((root / 'out').exists())


if __name__ == '__main__':
    unittest.main()
