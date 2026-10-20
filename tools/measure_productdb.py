#!/usr/bin/env python3
"""Private, source-bound ProductDB measurements using prebuilt production tools."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import signal
import sqlite3
import subprocess
import sys
import time
from contextlib import closing

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

ID = re.compile(r'[A-Za-z0-9][A-Za-z0-9_.-]*\Z')
SHA = re.compile(r'[0-9a-f]{64}\Z')
ROLES = {'Master', 'ApplicationProgram', 'Hardware', 'Catalog', 'Baggages',
         'Baggage', 'Signature', 'Unrecognized'}
ROOT = Path(__file__).resolve().parents[1]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def fields(value, keys):
    require(type(value) is dict and set(value) == set(keys.split()), 'invalid fields')


def text(value):
    return type(value) is str and bool(value) and '\0' not in value


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def absolute(value):
    require(text(str(value)) and Path(value).is_absolute(), 'absolute path required')
    return Path(value)


def safe_relative(value):
    require(text(value) and '\\' not in value and ':' not in value, 'unsafe relative path')
    parts = value.split('/')
    require(not any(p in ('', '.', '..') for p in parts) and not PurePosixPath(value).is_absolute(),
            'unsafe relative path')
    return value


def file_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def strict_json(path):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(Path(path).read_text(), object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def nonempty_list(value):
    require(type(value) is list and bool(value), 'nonempty list required')


def load_manifest(path):
    try:
        return _load_manifest(path)
    except (TypeError, KeyError, OSError) as error:
        raise ValueError('malformed manifest or unavailable input') from error


def _load_manifest(path):
    manifest = strict_json(absolute(path))
    fields(manifest, 'format inputRoot inputs cases')
    require(type(manifest['format']) is int and manifest['format'] == 1, 'unsupported manifest')
    root = absolute(manifest['inputRoot']).resolve(strict=True)
    require(root.is_dir(), 'input root is not a directory')
    nonempty_list(manifest['inputs'])
    inputs, paths = {}, set()
    for entry in manifest['inputs']:
        fields(entry, 'id path sha256 kind retainedPackages')
        require(type(entry['id']) is str and ID.fullmatch(entry['id']) and entry['id'] not in inputs,
                'invalid or duplicate input ID')
        relative = safe_relative(entry['path'])
        source = (root / relative).resolve(strict=True)
        require(source.is_relative_to(root) and source.is_file() and source not in paths,
                'input escapes root or duplicates a file')
        require(source.suffix == ('.knxprod' if entry['kind'] == 'package' else '.zip') and
                entry['kind'] in ('package', 'wrapper'), 'unsupported input kind or extension')
        require(type(entry['sha256']) is str and SHA.fullmatch(entry['sha256']) and
                file_sha(source) == entry['sha256'], 'input authentication failed')
        require(type(entry['retainedPackages']) is list, 'invalid package oracles')
        inner_paths = set()
        for package in entry['retainedPackages']:
            fields(package, 'innerPath sha256 members')
            inner = package['innerPath']
            require((inner is None if entry['kind'] == 'package' else
                     type(inner) is str and safe_relative(inner).endswith('.knxprod')),
                    'invalid inner path')
            require(inner not in inner_paths, 'duplicate package oracle')
            inner_paths.add(inner)
            require(type(package['sha256']) is str and SHA.fullmatch(package['sha256']),
                    'invalid package hash')
            if entry['kind'] == 'package':
                require(package['sha256'] == entry['sha256'], 'package hash differs from input')
            nonempty_list(package['members'])
            names = set()
            for ordinal, member in enumerate(package['members']):
                fields(member, 'ordinal path role sha256 sizeBytes')
                require(integer(member['ordinal']) and member['ordinal'] == ordinal,
                        'noncontiguous member ordinal')
                safe_relative(member['path'])
                require(member['path'] not in names, 'duplicate member path')
                names.add(member['path'])
                require(member['role'] in ROLES and integer(member['sizeBytes']) and
                        type(member['sha256']) is str and SHA.fullmatch(member['sha256']),
                        'invalid member oracle')
        if entry['kind'] == 'package':
            require(len(entry['retainedPackages']) <= 1, 'multiple standalone package oracles')
        inputs[entry['id']] = entry
        paths.add(source)
    nonempty_list(manifest['cases'])
    cases = set()
    for case in manifest['cases']:
        fields(case, 'id steps')
        require(type(case['id']) is str and ID.fullmatch(case['id']) and case['id'] not in cases,
                'invalid or duplicate case ID')
        cases.add(case['id'])
        nonempty_list(case['steps'])
        installed = set()
        for step in case['steps']:
            require(type(step) is dict, 'invalid step')
            if step.get('op') == 'install':
                fields(step, 'op input profile expectExit diagnosticSuffix preserveAll')
                require(step['input'] in inputs and step['profile'] in ('standard', 'large') and
                        type(step['expectExit']) is int and step['expectExit'] in (0, 1) and
                        type(step['preserveAll']) is bool, 'invalid install step')
                if step['expectExit'] == 0:
                    require(step['diagnosticSuffix'] is None and
                            bool(inputs[step['input']]['retainedPackages']), 'missing successful oracle')
                    installed.add(step['input'])
                else:
                    require(text(step['diagnosticSuffix']) and step['preserveAll'],
                            'refusal requires diagnostic and preservation')
            elif step.get('op') == 'resolve':
                fields(step, 'op packageInput programId expectStatus minimumSteps requiredIssueCodes')
                require(step['packageInput'] in installed and
                        inputs[step['packageInput']]['kind'] == 'package', 'uninstalled resolver input')
                require(text(step['programId']) and step['expectStatus'] in
                        ('expanded', 'partial', 'unavailable') and integer(step['minimumSteps']) and
                        type(step['requiredIssueCodes']) is list and
                        all(text(code) for code in step['requiredIssueCodes']), 'invalid resolve step')
            else:
                raise ValueError('unsupported operation')
    return manifest


def validate_output(output, input_root, checkout=ROOT):
    original = absolute(output)
    require(not original.exists() and not original.is_symlink(), 'output already exists')
    path = original.resolve()
    for other in (Path(input_root).resolve(), Path(checkout).resolve()):
        require(not (path.is_relative_to(other) or other.is_relative_to(path)), 'overlapping output')
    return path


def owned_environment(root):
    from tools.user_data_guard import isolated_environment
    root = absolute(root)
    root.mkdir(mode=0o700, parents=True, exist_ok=False)
    env = isolated_environment(dict(os.environ), root)
    for variable in ('HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME',
                     'XDG_STATE_HOME', 'XDG_RUNTIME_DIR', 'TMPDIR'):
        directory = root / variable.lower().replace('_', '-')
        directory.mkdir(mode=0o700, exist_ok=True)
        directory.chmod(0o700)
        env[variable] = str(directory)
    return env


def install_argv(knx, source, database, profile):
    require(profile in ('standard', 'large'), 'unsupported profile')
    return [str(knx), 'products', 'ingest', str(source), '--product-db', str(database)] + (
        ['--allow-large-package'] if profile == 'large' else [])


def outcome(stage, log, step):
    result = {'expected': False, 'diagnosticMatched': None, 'resolveResult': None}
    if step['op'] == 'install':
        diagnostic = step['diagnosticSuffix']
        if diagnostic is not None:
            result['diagnosticMatched'] = Path(log).read_text(errors='replace').rstrip('\r\n').endswith(diagnostic)
        result['expected'] = stage.get('exit') == step['expectExit'] and (
            diagnostic is None or result['diagnosticMatched'])
    elif stage.get('exit') == 0:
        try:
            summary = strict_json(log)
            fields(summary, 'format operation schema result')
            require(type(summary['format']) is int and summary['format'] == 1 and
                    summary['operation'] == 'resolve-ap1' and summary['schema'] == 28,
                    'invalid resolver envelope')
            resolved = summary['result']
            fields(resolved, 'status executable steps unplaced issueCodes omittedIssueCount')
            require(resolved['status'] in ('expanded', 'partial', 'unavailable') and
                    resolved['executable'] is False and integer(resolved['steps']) and
                    integer(resolved['unplaced']) and integer(resolved['omittedIssueCount']) and
                    type(resolved['issueCodes']) is list and all(text(c) for c in resolved['issueCodes']),
                    'invalid resolver result')
            result['resolveResult'] = resolved
            result['expected'] = (resolved['status'] == step['expectStatus'] and
                                  resolved['steps'] >= step['minimumSteps'] and
                                  set(step['requiredIssueCodes']) <= set(resolved['issueCodes']))
        except (ValueError, TypeError, KeyError):
            pass
    return result


def source_identity(root):
    from tools.web_gate_support import source_fingerprint
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root))
    return commit, source_fingerprint(root), dirty


def load_build_receipt(path, identity, executables):
    build = strict_json(path)
    fields(build, 'format success sourceCommit sourceFingerprint profile executables')
    require(type(build['format']) is int and build['format'] == 1 and build['success'] is True and
            build['profile'] == 'release', 'unsupported or unsuccessful build receipt')
    require(build['sourceCommit'] == identity[0] and build['sourceFingerprint'] == identity[1] and
            not identity[2], 'build source is not the clean current source')
    fields(build['executables'], 'knx probe supervisor')
    fields(executables, 'knx probe supervisor')
    for name, binary in executables.items():
        fields(build['executables'][name], 'sha256')
        require(type(build['executables'][name]['sha256']) is str and
                SHA.fullmatch(build['executables'][name]['sha256']) and
                file_sha(binary) == build['executables'][name]['sha256'], 'build binary mismatch')
    return build


def save_json(path, data):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(data, ensure_ascii=True, indent=2) + '\n')
    temporary.replace(path)


def observer_identity():
    require(sys.platform.startswith('linux') and hasattr(os, 'wait4'), 'unsupported RSS observer')
    with closing(sqlite3.connect(':memory:')) as db:
        try:
            db.execute('SELECT count(*) FROM dbstat WHERE aggregate=FALSE').fetchone()
        except sqlite3.DatabaseError as error:
            raise ValueError('dbstat unavailable') from error
        return {'pythonVersion': platform.python_version(), 'sqliteVersion': sqlite3.sqlite_version,
                'sqliteSourceId': db.execute('SELECT sqlite_source_id()').fetchone()[0],
                'compileOptions': [row[0] for row in db.execute('PRAGMA compile_options')], 'dbstat': True,
                'rssMethod': 'native-supervisor-wait4-target', 'rssUnit': 'KiB',
                'rssScope': 'whole-native-launched-target',
                'nativePreExecFloor': 'fresh-native-supervisor-not-subtracted',
                'samplingIntervalMs': None, 'elapsedScope': 'external-step-with-checkpoint-overhead'}


def stage_inputs(manifest, output):
    root = output / 'inputs'
    root.mkdir(mode=0o700)
    staged = {}
    for entry in manifest['inputs']:
        target = root / (entry['id'] + Path(entry['path']).suffix)
        shutil.copyfile(Path(manifest['inputRoot']) / entry['path'], target)
        require(file_sha(target) == entry['sha256'], 'staged input authentication failed')
        target.chmod(0o400)
        staged[entry['id']] = target
    return staged


def setup_database(probe, database, output, env):
    log = output / 'init.log'
    command = [str(probe), 'init', '--product-db', str(database)]
    with log.open('xb') as stream:
        child = subprocess.run(command, cwd=output, env=env, stdout=stream, stderr=subprocess.STDOUT)
    save_json(output / 'setup.json', {'measured': False, 'operation': 'init', 'command': command,
                                     'exit': child.returncode})
    require(child.returncode == 0, 'setup process failed')
    summary = strict_json(log)
    fields(summary, 'format operation schema sqliteVersion')
    require(type(summary['format']) is int and summary['format'] == 1 and summary['operation'] == 'init'
            and type(summary['schema']) is int and summary['schema'] == 28 and text(summary['sqliteVersion']),
            'invalid current-schema setup proof')


def observed_step(run, helper, name, command, cwd, env):
    from tools.productdb_process_observer import observed_step as observe
    return observe(run, helper, name, command, cwd, env)


def process_result(stage):
    exit_code = stage.get('exit')
    return {'pid': stage.get('pid'), 'exit': exit_code,
            'signal': -exit_code if type(exit_code) is int and exit_code < 0 else None,
            'elapsedSeconds': stage.get('wallSeconds'), 'cpuSeconds': stage.get('cpuSeconds'),
            'peakRssKiB': stage.get('peakRssKiB'), 'waitStatus': stage.get('waitStatus'),
            'supervisorPid': stage.get('supervisorPid'), 'observerError': stage.get('observerError')}


def names_in(database):
    from tools.productdb_measurement_accounting import reader
    with reader(database) as db:
        return set(db.execute('SELECT package_sha256,source_name FROM package_source_name'))


def run_study(args):
    """Run a fixed manifest serially; errors never manufacture accepted observations."""
    from tools import productdb_measurement_accounting as accounting
    from tools.run_package_gate import workspace_lease
    from tools.web_gate_support import MeasuredRun
    manifest_path, build_path = absolute(args.manifest), absolute(args.build_receipt)
    manifest = load_manifest(manifest_path)
    require(integer(args.repetitions, 1), 'positive repetitions required')
    require(type(args.lease_timeout) in (int, float) and 0 <= args.lease_timeout < float('inf'),
            'invalid lease timeout')
    output = validate_output(args.output, manifest['inputRoot'])
    executables = {name: absolute(getattr(args, name)).resolve(strict=True) for name in ('knx', 'probe', 'supervisor')}
    for binary in executables.values():
        require(binary.is_file() and os.access(binary, os.X_OK), 'executable required')
    identity = source_identity(ROOT)
    build = load_build_receipt(build_path, identity, executables)
    observer = observer_identity()
    manifest_bytes = manifest_path.read_bytes()
    build_bytes = build_path.read_bytes()
    # The authenticated build hashes remain authoritative. A later snapshot
    # cannot promote an executable replaced after validation into the baseline.
    binary_hashes = {name: build['executables'][name]['sha256'] for name in executables}
    require(all(file_sha(path) == binary_hashes[name] for name, path in executables.items()),
            'build binary mismatch before study')
    expected = [f'r{rep}-c{ci}-s{si}' for rep in range(args.repetitions)
                for ci, case in enumerate(manifest['cases']) for si, _ in enumerate(case['steps'])]
    old_umask = os.umask(0o077)
    try:
        output.mkdir(mode=0o700, parents=True, exist_ok=False)
        save_json(output / 'manifest.json', manifest)
        (output / 'manifest.original.json').write_bytes(manifest_bytes)
        (output / 'build-receipt.json').write_bytes(build_bytes)
        # These are the expanded source-bound profiles, not resource peak claims.
        save_json(output / 'profiles.json', {
            'schema': 28, 'rawInputBytes': 1000000000, 'maxMemberBytes': 1000000000,
            'standard': {'maxExpandedBytes': 4294967296},
            'large': {'maxExpandedBytes': 17179869184}})
        study = {'format': 1, 'complete': False, 'accepted': False,
                 'source': {'commit': identity[0], 'fingerprintStart': identity[1],
                            'fingerprintEnd': None, 'dirtyAtStart': identity[2],
                            'buildReceiptSha256': hashlib.sha256(build_bytes).hexdigest()},
                 'executables': {name: {'path': str(path), 'sha256Start': binary_hashes[name],
                                        'sha256End': None} for name, path in executables.items()},
                 'observer': observer, 'repetitions': args.repetitions,
                 'cachePolicy': 'uncontrolled-os-cache-no-flush-no-warmup', 'leaseWaitSeconds': 0.0,
                 'observations': [], 'errors': []}
        save_json(output / 'receipt.json', study)
        run = MeasuredRun(output / 'processes', expected)
        staged = {}
        lease_started = None
        try:
            staged = stage_inputs(manifest, output)
            lease_started = time.monotonic()
            with workspace_lease(ROOT, run, args.lease_timeout):
                study['leaseWaitSeconds'] = run.receipt['workspaceLeaseWaitSeconds']
                for rep in range(args.repetitions):
                    for ci, case in enumerate(manifest['cases']):
                        case_output = output / f'r{rep}-c{ci}'
                        case_output.mkdir(mode=0o700)
                        database = case_output / 'catalogue.sqlite'
                        env = owned_environment(case_output / 'environment')
                        setup_database(executables['probe'], database, case_output, env)
                        admitted = []
                        entries = {entry['id']: entry for entry in manifest['inputs']}
                        before = accounting.snapshot(database)
                        require(before['pragmas']['userVersion'] == 28, 'unexpected on-disk schema')
                        require(all(accounting.integrity(database).values()) and
                                all(accounting.verify_sources(database, []).values()), 'nonempty or invalid baseline')
                        logical_before = accounting.logical_snapshot(database, case_output / 'blank-rows')
                        save_json(case_output / 'blank.json', before)
                        for si, step in enumerate(case['steps']):
                            name = f'r{rep}-c{ci}-s{si}'
                            input_id = step['input'] if step['op'] == 'install' else step['packageInput']
                            observation = {'caseId': case['id'], 'repetition': rep, 'step': si,
                                'operation': 'install' if step['op'] == 'install' else 'resolve-ap1',
                                'profile': step.get('profile'), 'inputId': input_id, 'processStage': name,
                                'process': process_result({}),
                                'outcome': {'expected': False, 'diagnosticMatched': None, 'resolveResult': None},
                                'before': before, 'after': None, 'delta': None,
                                'integrity': dict.fromkeys(('sqliteOk', 'foreignKeysOk', 'packagesExact',
                                    'sourcesExact', 'memberIndexExact', 'preservationExact')),
                                'accepted': False}
                            study['observations'].append(observation)
                            save_json(output / 'receipt.json', study)
                            entry, source = entries[input_id], staged[input_id]
                            was_installed = any(e['sha256'] == entry['sha256'] for e, _ in admitted)
                            alias_names = names_in(database) if was_installed and entry['kind'] == 'package' else None
                            if step['op'] == 'install':
                                command = install_argv(executables['knx'], source, database, step['profile'])
                            else:
                                command = [str(executables['probe']), 'resolve-ap1', '--product-db', str(database),
                                           '--package-sha256', entry['sha256'], '--program-id', step['programId']]
                            stage = observed_step(run, executables['supervisor'], name, command, case_output, env)
                            observation['process'] = process_result(stage)
                            observation['outcome'] = outcome(stage, run.output / (name + '.log'), step)
                            # After reads are unconditional, including nonzero/signal outcomes.
                            after = accounting.snapshot(database)
                            observation['after'] = after
                            observation['delta'] = accounting.delta(before, after)
                            logical_after = accounting.logical_snapshot(database, case_output / f's{si}-rows')
                            if step['op'] == 'install' and stage['exit'] == 0 and step['expectExit'] == 0:
                                admitted.append((entry, source))
                            checks = {**accounting.integrity(database), **accounting.verify_sources(database, admitted)}
                            preserve = step['op'] == 'resolve' or step.get('preserveAll')
                            checks['preservationExact'] = accounting.preservation_exact(
                                database, logical_before, logical_after, preserve,
                                alias_names if stage['exit'] == 0 else None, (entry['sha256'], str(source)))
                            observation['integrity'] = checks
                            metrics = observation['process']
                            observed = (integer(metrics['pid'], 1) and type(metrics['exit']) is int and
                                        integer(metrics['peakRssKiB'], 1) and
                                        all(type(metrics[k]) in (int, float) and metrics[k] >= 0
                                            for k in ('elapsedSeconds', 'cpuSeconds')))
                            require(observed, 'missing native target resource observation')
                            observation['accepted'] = bool(observation['outcome']['expected'] and
                                all(checks[key] is True for key in ('sqliteOk', 'foreignKeysOk',
                                    'packagesExact', 'sourcesExact', 'memberIndexExact')) and
                                checks['preservationExact'] is not False and
                                not any(after['retained'][key] for key in ('packageLengthMismatchCount',
                                    'sourceLengthMismatchCount', 'memberLengthMismatchCount')))
                            save_json(output / 'receipt.json', study)
                            before, logical_before = after, logical_after
        except (Exception, KeyboardInterrupt) as error:
            if lease_started is not None and 'workspaceLeaseWaitSeconds' not in run.receipt:
                study['leaseWaitSeconds'] = time.monotonic() - lease_started
            study['errors'].append({'code': 'observation-incomplete', 'detail': str(error) or type(error).__name__})
        finally:
            def end_check(action):
                try:
                    action()
                except Exception as error:
                    study['errors'].append({'code': 'identity-invalid', 'detail': str(error)})
            def source_end():
                end = source_identity(ROOT)
                study['source']['fingerprintEnd'] = end[1]
                require(end == identity and not end[2], 'source drift')
            end_check(source_end)
            for name, path in executables.items():
                def binary_end(name=name, path=path):
                    digest = file_sha(path)
                    study['executables'][name]['sha256End'] = digest
                    require(digest == binary_hashes[name], 'binary drift')
                end_check(binary_end)
            end_check(lambda: require(manifest_path.read_bytes() == manifest_bytes and
                                      build_path.read_bytes() == build_bytes, 'manifest or build receipt drift'))
            for entry in manifest['inputs']:
                end_check(lambda entry=entry: require(
                    file_sha(Path(manifest['inputRoot']) / entry['path']) == entry['sha256'] and
                    entry['id'] in staged and file_sha(staged[entry['id']]) == entry['sha256'], 'input drift'))
            run.finish(identity[1], study['source']['fingerprintEnd'])
            study['complete'] = (len(study['observations']) == len(expected) and
                                 all(o['after'] is not None and o['delta'] is not None and
                                     o['process']['exit'] is not None for o in study['observations']) and
                                 not any(e['code'] == 'observation-incomplete' for e in study['errors']))
            study['accepted'] = study['complete'] and not study['errors'] and all(
                o['accepted'] for o in study['observations'])
            save_json(output / 'receipt.json', study)
            try:
                save_json(output / 'receipt-binding.json', {
                    'format': 1, 'receiptSha256': file_sha(output / 'receipt.json'),
                    'artifacts': {str(path.relative_to(output)): file_sha(path)
                                  for path in sorted(output.rglob('*')) if path.is_file() and
                                  path.name not in ('receipt-binding.json', 'receipt-binding.tmp') and
                                  not path.is_relative_to(output / 'environment')}})
            except (Exception, KeyboardInterrupt) as error:
                study.update(complete=False, accepted=False)
                study['errors'].append({'code': 'artifact-binding-incomplete', 'detail': str(error)})
                save_json(output / 'receipt.json', study)
        return study
    finally:
        os.umask(old_umask)


class PrivateArgumentParser(argparse.ArgumentParser):
    def error(self, message):
        self.exit(2, 'ProductDB measurement refused: invalid arguments.\n')


def main(argv=None):
    parser = PrivateArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True, parser_class=PrivateArgumentParser)
    fixtures = commands.add_parser('fixtures')
    fixtures.add_argument('--output', required=True)
    run = commands.add_parser('run')
    for name in ('knx', 'probe', 'supervisor', 'build-receipt', 'manifest', 'output'):
        run.add_argument('--' + name, required=True)
    run.add_argument('--repetitions', required=True, type=int)
    run.add_argument('--lease-timeout', type=float, default=5400.0)
    args = parser.parse_args(argv)
    previous = signal.getsignal(signal.SIGTERM)
    def interrupt(_signal, _frame):
        raise KeyboardInterrupt('terminated')
    signal.signal(signal.SIGTERM, interrupt)
    try:
        if args.command == 'fixtures':
            output = absolute(args.output)
            require(not output.exists(), 'fixture output already exists')
            require(not output.resolve().is_relative_to(ROOT), 'fixture output overlaps checkout')
            from tools.productdb_measurement_fixtures import generate
            generate(output)
            print('Synthetic fixtures generated (private output).')
            return 0
        study = run_study(args)
        print('ProductDB measurement: ' + ('accepted.' if study['accepted'] else 'not accepted.'))
        return 0 if study['accepted'] else 1
    except (Exception, KeyboardInterrupt):
        print('ProductDB measurement refused; no accepted result.', file=sys.stderr)
        return 1
    finally:
        signal.signal(signal.SIGTERM, previous)


if __name__ == '__main__':
    raise SystemExit(main())
