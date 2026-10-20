"""Protocol controls, not acceptance of synthetic helper RSS as native evidence."""
from __future__ import annotations

import copy
import importlib
import json
import math
from pathlib import Path
import signal
import sys
from tempfile import TemporaryDirectory
import unittest


def observer(test):
    from tools import measure_productdb
    test.assertTrue(hasattr(measure_productdb, 'observed_step'),
                    'native-target observer integration is missing')
    return importlib.import_module('tools.productdb_process_observer')


def synthetic_helper(root):
    """Actual Python protocol control only; deliberately NOT the native observer."""
    helper = root / 'synthetic-observer'
    helper.write_text('#!' + sys.executable + '\n' + '''
import json,os,subprocess,sys,time
receipt=sys.argv[2]
assert sys.argv[1]=='--receipt' and sys.argv[3]=='--'
with open(receipt,'x') as stream:
    start=time.monotonic()
    child=subprocess.Popen(sys.argv[4:],stdin=subprocess.DEVNULL)
    json.dump({'format':1,'complete':False,'pid':child.pid},stream)
    stream.flush()
    _,status,usage=os.wait4(child.pid,0)
    child.returncode=os.waitstatus_to_exitcode(status)
    result={'format':1,'complete':True,'pid':child.pid,'waitStatus':status,
        'exit':child.returncode,'signal':-child.returncode if child.returncode<0 else None,
        'peakRssKiB':usage.ru_maxrss,'cpuSeconds':usage.ru_utime+usage.ru_stime,
        'elapsedSeconds':time.monotonic()-start}
    stream.seek(0);stream.truncate();json.dump(result,stream)
raise SystemExit(child.returncode if child.returncode>=0 else 128-child.returncode)
''')
    helper.chmod(0o700)
    return helper


class NativeProtocolTests(unittest.TestCase):
    def test_target_proof_does_not_credit_outer_wrapper_pid_rss_or_elapsed(self):
        api = observer(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'proof.json'
            proof = {'format': 1, 'complete': True, 'pid': 72, 'waitStatus': 256,
                     'exit': 1, 'signal': None, 'peakRssKiB': 1234,
                     'cpuSeconds': 0.001, 'elapsedSeconds': 0.003}
            path.write_text(json.dumps(proof))
            wrapper = {'pid': 71, 'exit': 1, 'peakRssKiB': 999999,
                       'cpuSeconds': 50.0, 'wallSeconds': 60.0}
            actual = api.target_observation(wrapper, path)
            self.assertEqual(actual['pid'], 72)
            self.assertEqual(actual['peakRssKiB'], 1234)
            self.assertEqual(actual['wallSeconds'], 0.003)
            self.assertEqual(actual['cpuSeconds'], 0.001)
            self.assertEqual(actual['exit'], 1)
            self.assertEqual(actual['waitStatus'], 256)
            self.assertEqual(actual['supervisorPid'], 71)

    def test_missing_malformed_incomplete_and_inconsistent_proofs_refuse(self):
        api = observer(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'proof.json'
            wrapper = {'pid': 71, 'exit': 0, 'peakRssKiB': 123, 'cpuSeconds': 0.1, 'wallSeconds': 0.2}
            good = {'format': 1, 'complete': True, 'pid': 72, 'waitStatus': 0,
                    'exit': 0, 'signal': None, 'peakRssKiB': 512,
                    'cpuSeconds': 0.001, 'elapsedSeconds': 0.002}
            with self.assertRaises((OSError, ValueError)):
                api.target_observation(wrapper, path)
            for field, value in [('format', True), ('complete', False), ('pid', 71),
                                 ('pid', True), ('pid', 0), ('waitStatus', 256),
                                 ('waitStatus', 127), ('exit', True), ('signal', 15),
                                 ('peakRssKiB', None), ('peakRssKiB', True),
                                 ('cpuSeconds', math.nan), ('elapsedSeconds', math.inf),
                                 ('extra', 1)]:
                bad = {**good, field: value}
                path.write_text(json.dumps(bad))
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    api.target_observation(wrapper, path)
            path.write_text('{"format":1,"format":1}')
            with self.assertRaises(ValueError):
                api.target_observation(wrapper, path)
            path.write_text(json.dumps(good))
            with self.assertRaises(ValueError):
                api.target_observation({**wrapper, 'exit': 1}, path)
            with self.assertRaises(ValueError):
                api.target_observation({**wrapper, 'peakRssKiB': None}, path)

    def test_signal_target_remains_negative_and_wrapper_signal_is_not_target_status(self):
        api = observer(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'proof.json'
            proof = {'format': 1, 'complete': True, 'pid': 72,
                     'waitStatus': signal.SIGTERM, 'exit': -signal.SIGTERM,
                     'signal': signal.SIGTERM, 'peakRssKiB': 512,
                     'cpuSeconds': 0.0, 'elapsedSeconds': 0.001}
            path.write_text(json.dumps(proof))
            wrapper = {'pid': 71, 'exit': 128 + signal.SIGTERM, 'peakRssKiB': 123,
                       'cpuSeconds': 0.1, 'wallSeconds': 0.2}
            self.assertEqual(api.target_observation(wrapper, path)['exit'], -signal.SIGTERM)
            with self.assertRaises(ValueError):
                api.target_observation({**wrapper, 'exit': -signal.SIGTERM}, path)

    def test_real_protocol_control_keeps_target_output_raw_wrapper_and_target_separate(self):
        api = observer(self)
        from tools.web_gate_support import MeasuredRun
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            helper = synthetic_helper(root)
            run = MeasuredRun(root / 'processes')
            target = api.observed_step(run, helper, 'refused',
                [sys.executable, '-c', 'print("named refusal");raise SystemExit(1)'], root, None)
            self.assertEqual(target['exit'], 1)
            self.assertEqual((run.output / 'refused.log').read_text().strip(), 'named refusal')
            raw = run.receipt['stages'][0]
            self.assertEqual(raw['exit'], 1)
            self.assertNotEqual(raw['pid'], target['pid'])
            self.assertEqual(raw['pid'], target['supervisorPid'])
            self.assertEqual(raw['command'][0], str(helper))
            self.assertTrue((run.output / 'refused.target.json').is_file())
            self.assertFalse(run.finish('same', 'same'))

    def test_completed_wrapper_without_target_proof_never_becomes_target_metrics(self):
        api = observer(self)
        from tools.web_gate_support import MeasuredRun
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            helper = root / 'broken-observer'
            helper.write_text('#!' + sys.executable + '\nraise SystemExit(0)\n')
            helper.chmod(0o700)
            run = MeasuredRun(root / 'processes')
            target = api.observed_step(run, helper, 'broken', ['/usr/bin/true'], root, None)
            self.assertIsNone(target['exit'])
            self.assertIsNone(target['peakRssKiB'])
            self.assertTrue(target['observerError'])
            self.assertEqual(run.receipt['stages'][0]['exit'], 0)


if __name__ == '__main__':
    unittest.main()
