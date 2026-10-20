"""Synthetic source oracles; importer/resolver acceptance is a separate run."""
from __future__ import annotations

import hashlib
from contextlib import chdir
import io
import json
from pathlib import Path
import re
import shutil
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET
import zipfile


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class MeasurementFixtures(unittest.TestCase):
    def test_manifest_authenticates_every_external_package_and_member(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        with TemporaryDirectory() as temporary:
            output = Path(temporary) / 'inputs'
            manifest_path = generate(output)
            self.assertEqual(manifest_path, output / 'manifest.json')
            manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
            self.assertEqual(set(manifest), {'format', 'inputRoot', 'inputs', 'cases'})
            self.assertEqual(manifest['format'], 1)
            self.assertEqual(manifest['inputRoot'], str(output.resolve()))
            self.assertTrue(manifest['inputs'])
            self.assertTrue(manifest['cases'])
            ids = [item['id'] for item in manifest['inputs']]
            self.assertEqual(len(ids), len(set(ids)))
            for item in manifest['inputs']:
                with self.subTest(input=item['id']):
                    self.assertEqual(set(item), {'id', 'path', 'sha256', 'kind', 'retainedPackages'})
                    self.assertRegex(item['id'], r'^[A-Za-z0-9][A-Za-z0-9_.-]*$')
                    path = Path(item['path'])
                    self.assertFalse(path.is_absolute())
                    self.assertNotIn('..', path.parts)
                    raw = (output / path).read_bytes()
                    self.assertEqual(item['sha256'], digest(raw))
                    self.assertIn(item['kind'], {'package', 'wrapper'})
                    for package in item['retainedPackages']:
                        self.assertEqual(set(package), {'innerPath', 'sha256', 'members'})
                        package_bytes = raw
                        if item['kind'] == 'wrapper':
                            with zipfile.ZipFile(io.BytesIO(raw)) as outer:
                                package_bytes = outer.read(package['innerPath'])
                        else:
                            self.assertIsNone(package['innerPath'])
                        self.assertEqual(package['sha256'], digest(package_bytes))
                        with zipfile.ZipFile(io.BytesIO(package_bytes)) as archive:
                            self.assertIsNone(archive.testzip())
                            files = [member for member in archive.infolist() if not member.is_dir()]
                            self.assertEqual(len(package['members']), len(files))
                            for ordinal, (oracle, member) in enumerate(zip(package['members'], files)):
                                self.assertEqual(set(oracle), {'ordinal', 'path', 'role', 'sha256', 'sizeBytes'})
                                expected_role = ('Master' if ordinal == 0 else
                                                 'ApplicationProgram' if ordinal == 1 else 'Signature')
                                self.assertEqual(oracle, {
                                    'ordinal': ordinal, 'path': member.filename, 'role': expected_role,
                                    'sha256': digest(archive.read(member)), 'sizeBytes': member.file_size,
                                })


    def test_ladder_first_complete_rows_reach_decoded_mib_targets(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        ns = {'k': 'http://knx.org/xml/project/20'}
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            inputs = {item['id']: item for item in manifest['inputs']}
            for mib in (1, 4, 16):
                input_id = f'ladder-{mib}mib'
                self.assertIn(input_id, inputs, 'missing meaningful size ladder input')
                item = inputs[input_id]
                with zipfile.ZipFile(root / item['path']) as archive:
                    member = item['retainedPackages'][0]['members'][1]
                    data = archive.read(member['path'])
                    tree = ET.fromstring(data)
                    parameters = tree.findall('.//k:Parameter', ns)
                    encoded_rows = re.findall(rb'<Parameter [^<>]*/>', data)
                    target = mib * 1024 * 1024
                    self.assertGreaterEqual(len(data), target)
                    self.assertLess(len(data) - len(encoded_rows[-1]), target)
                    self.assertEqual(len(parameters), len(encoded_rows))
                    self.assertGreater(len(parameters), 1000)
                    self.assertEqual(len({p.attrib['Id'] for p in parameters}), len(parameters))
                    for index, parameter in enumerate(parameters):
                        self.assertTrue(parameter.attrib['Id'].endswith(f'_P-{index}'))
                        self.assertEqual(parameter.attrib['Text'], 'X' * 512 + '-λ')
                        self.assertEqual(parameter.attrib['Value'], '0')
                        self.assertEqual(parameter.attrib['FutureMeasure'], 'first-λ')
                        self.assertTrue(parameter.attrib['ParameterType'].endswith('_PT-N'))
                    number = tree.find('.//k:TypeNumber', ns)
                    self.assertIsNotNone(number)
                    self.assertEqual(number.attrib, {'SizeInBit': '8', 'Type': 'unsignedInt',
                                                     'minInclusive': '0', 'maxInclusive': '255'})
                    self.assertNotIn(b'<!--', data)
                recipe = json.loads((root / 'fixture-recipe.json').read_text(encoding='utf-8'))
                expected = recipe['programs'][input_id]
                self.assertEqual(expected['targetBytes'], target)
                self.assertEqual(expected['actualBytes'], len(data))
                self.assertEqual(expected['parameterRows'], len(parameters))
                self.assertEqual(expected['lastRowBytes'], len(encoded_rows[-1]))
                self.assertEqual(expected['textUtf8Bytes'], len(parameters) * len(('X' * 512 + '-λ').encode()))
                self.assertEqual(expected['unknownSubset'], [
                    {'name': 'FutureMeasure', 'occurrences': len(parameters), 'sample': 'first-λ'}])
                self.assertEqual(expected['sizing'], 'first-complete-row-at-target')


    def test_distinct_unknown_keys_keep_the_same_sixteen_occurrences(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        ns = {'k': 'http://knx.org/xml/project/20'}
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            inputs = {item['id']: item for item in manifest['inputs']}
            self.assertIn('evidence-distinct', inputs, 'missing distinct-key companion')
            recipe = json.loads((root / 'fixture-recipe.json').read_text(encoding='utf-8'))
            for input_id, distinct in [('shared-a', False), ('evidence-distinct', True)]:
                item = inputs[input_id]
                with zipfile.ZipFile(root / item['path']) as archive:
                    parameters = ET.fromstring(archive.read(item['retainedPackages'][0]['members'][1]['path'])).findall('.//k:Parameter', ns)
                self.assertEqual(len(parameters), 16)
                occurrences = {}
                for index, parameter in enumerate(parameters):
                    names = [name for name in parameter.attrib if name.startswith('FutureMeasure')]
                    self.assertEqual(names, [f'FutureMeasure{index:02d}' if distinct else 'FutureMeasure'])
                    self.assertEqual(parameter.attrib[names[0]], 'first-λ')
                    occurrences[names[0]] = occurrences.get(names[0], 0) + 1
                self.assertEqual(sum(occurrences.values()), 16)
                self.assertEqual(len(occurrences), 16 if distinct else 1)
                self.assertEqual(recipe['programs'][input_id]['unknownSubset'], [
                    {'name': name, 'occurrences': count, 'sample': 'first-λ'}
                    for name, count in occurrences.items()])


    def test_shared_archives_reuse_decoded_bytes_and_retry_identity(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            inputs = {item['id']: item for item in manifest['inputs']}
            self.assertTrue({'shared-a', 'shared-b', 'shared-alias'} <= inputs.keys(),
                            'missing shared archive/retry/alias fixtures')
            a, b, alias = (inputs[key] for key in ('shared-a', 'shared-b', 'shared-alias'))
            self.assertNotEqual(a['path'], alias['path'])
            self.assertEqual(a['sha256'], alias['sha256'])
            self.assertNotEqual(a['sha256'], b['sha256'])
            self.assertEqual((root / a['path']).read_bytes(), (root / alias['path']).read_bytes())
            self.assertEqual(a['retainedPackages'][0]['members'], b['retainedPackages'][0]['members'])
            decoded = []
            comments = []
            for item in (a, b):
                with zipfile.ZipFile(root / item['path']) as archive:
                    decoded.append([(info.filename, archive.read(info)) for info in archive.infolist()])
                    comments.append(archive.comment)
            self.assertEqual(decoded[0], decoded[1])
            self.assertEqual(comments, [b'', b'fictional-measurement-shared-b'])
            case = next(item for item in manifest['cases'] if item['id'] == 'shared-retry-alias')
            self.assertEqual(case, {'id': 'shared-retry-alias', 'steps': [
                {'op': 'install', 'input': input_id, 'profile': 'standard', 'expectExit': 0,
                 'diagnosticSuffix': None, 'preserveAll': preserve}
                for input_id, preserve in [('shared-a', False), ('shared-a', True),
                                           ('shared-b', False), ('shared-b', True),
                                           ('shared-alias', False)]
            ]})


    def test_seeded_wrapper_has_new_first_inner_and_last_truncated_xml(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        ns = {'k': 'http://knx.org/xml/project/20'}
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            inputs = {item['id']: item for item in manifest['inputs']}
            self.assertTrue('rollback-wrapper' in inputs, 'missing late-inner rollback wrapper')
            wrapper = inputs['rollback-wrapper']
            self.assertEqual(wrapper['kind'], 'wrapper')
            self.assertEqual(wrapper['retainedPackages'], [])
            with zipfile.ZipFile(root / inputs['shared-a']['path']) as seed:
                seed_program = ET.fromstring(seed.read(inputs['shared-a']['retainedPackages'][0]['members'][1]['path']))
                seed_id = seed_program.find('.//k:ApplicationProgram', ns).attrib['Id']
            with zipfile.ZipFile(root / wrapper['path']) as outer:
                self.assertIsNone(outer.testzip())
                self.assertEqual(outer.namelist(), ['01-new.knxprod', '02-refused.knxprod', 'Manual.txt'])
                inner_ids = []
                for name in ('01-new.knxprod', '02-refused.knxprod'):
                    with zipfile.ZipFile(io.BytesIO(outer.read(name))) as inner:
                        self.assertIsNone(inner.testzip())
                        files = inner.namelist()
                        ET.fromstring(inner.read('knx_master.xml'))
                        app = ET.fromstring(inner.read(files[1]))
                        inner_ids.append(app.find('.//k:ApplicationProgram', ns).attrib['Id'])
                        self.assertGreater(len(app.findall('.//k:Parameter', ns)), 0)
                        if name == '02-refused.knxprod':
                            self.assertEqual(files[-1], 'M-0001/late.xml')
                            self.assertEqual(inner.read(files[-1]), b'<Broken>')
                            with self.assertRaises(ET.ParseError):
                                ET.fromstring(inner.read(files[-1]))
                        else:
                            for path in files:
                                if path.endswith('.xml'):
                                    ET.fromstring(inner.read(path))
                self.assertEqual(len({seed_id, *inner_ids}), 3)
            case = next(item for item in manifest['cases'] if item['id'] == 'seeded-rollback')
            self.assertEqual(case['steps'], [
                {'op': 'install', 'input': 'shared-a', 'profile': 'standard',
                 'expectExit': 0, 'diagnosticSuffix': None, 'preserveAll': False},
                {'op': 'install', 'input': 'rollback-wrapper', 'profile': 'standard',
                 'expectExit': 1, 'preserveAll': True, 'diagnosticSuffix':
                 'inner product package 02-refused.knxprod: M-0001/late.xml: incomplete XML document'},
            ])


    def test_resolver_sources_target_bytes_with_persisted_text_and_nonempty_ap1(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        ns = {'k': 'http://knx.org/xml/project/20'}
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            inputs = {item['id']: item for item in manifest['inputs']}
            resolver_ids = ['resolver-small', 'resolver-near-limit', 'resolver-oversized']
            self.assertTrue(set(resolver_ids) <= inputs.keys(), 'missing resolver fixture trio')
            recipe = json.loads((root / 'fixture-recipe.json').read_text(encoding='utf-8'))
            seen_ids = set()
            for input_id in resolver_ids:
                item = inputs[input_id]
                with zipfile.ZipFile(root / item['path']) as archive:
                    master = ET.fromstring(archive.read('knx_master.xml'))
                    data = archive.read(item['retainedPackages'][0]['members'][1]['path'])
                    tree = ET.fromstring(data)
                program = tree.find('.//k:ApplicationProgram', ns)
                seen_ids.add(program.attrib['Id'])
                self.assertEqual(program.attrib['MaskVersion'], 'MV-07B0')
                self.assertEqual(program.attrib['LoadProcedureStyle'], 'MergedProcedure')
                sequence = master.find('k:MasterData/k:MaskVersions/k:MaskVersion/k:HawkConfigurationData/k:Procedures/k:Procedure', ns)
                self.assertIsNotNone(sequence, 'AP1 mask must be inside MasterData, not legacy test-helper root')
                self.assertEqual(sequence.attrib, {'ProcedureType': 'Load', 'ProcedureSubType': 'ap1', 'Access': 'remote local2'})
                local = lambda node: node.tag.rsplit('}', 1)[-1]
                self.assertEqual([local(node) for node in sequence], [
                    'LdCtrlConnect', 'LdCtrlMerge', 'LdCtrlLoad', 'LdCtrlMerge',
                    'LdCtrlMerge', 'LdCtrlMerge', 'LdCtrlMerge', 'LdCtrlRestart'])
                self.assertEqual([node.attrib['MergeId'] for node in sequence if local(node) == 'LdCtrlMerge'], ['1', '2', '4', '6', '7'])
                fragments = tree.findall('.//k:LoadProcedures/k:LoadProcedure', ns)
                self.assertEqual([f.attrib for f in fragments], [{'MergeId': '2'}, {'MergeId': '4'}])
                self.assertEqual([local(f[0]) for f in fragments], ['LdCtrlRelSegment', 'LdCtrlWriteRelMem'])
                parameters = tree.findall('.//k:Parameter', ns)
                self.assertEqual(len(parameters), 16)
                self.assertEqual([p.attrib['Text'] for p in parameters[:-1]], ['X' * 512 + '-λ'] * 15)
                expected = recipe['programs'][input_id]
                self.assertEqual(expected['actualBytes'], len(data))
                self.assertEqual(expected['parameterRows'], 16)
                self.assertEqual(expected['textUtf8Bytes'], sum(len(p.attrib['Text'].encode()) for p in parameters))
                self.assertEqual(expected['xml']['nodes'], len(list(tree.iter())))
                self.assertEqual(expected['xml']['eventsIncludingEof'], len(re.findall(rb'<[^<>]*>', data)) + 1)
                self.assertLess(expected['xml']['nodes'], 65536)
                self.assertLess(expected['xml']['eventsIncludingEof'], 200000)
                self.assertLess(expected['xml']['maxDepth'], 64)
                self.assertLess(expected['xml']['maxRawAttributes'], 256)
                if input_id == 'resolver-near-limit':
                    self.assertEqual(len(data), 8 * 1024 * 1024 - 4096)
                    self.assertEqual(expected['targetBytes'], len(data))
                    self.assertEqual(expected['sizing'], 'final-persisted-text')
                    self.assertGreater(len(parameters[-1].attrib['Text']), 7 * 1024 * 1024)
                elif input_id == 'resolver-oversized':
                    self.assertEqual(len(data), 8 * 1024 * 1024 + 4096)
                else:
                    self.assertLess(len(data), 32 * 1024)
                case = next(case for case in manifest['cases'] if case['id'] == input_id)
                self.assertEqual(case['steps'][0], {
                    'op': 'install', 'input': input_id, 'profile': 'standard',
                    'expectExit': 0, 'diagnosticSuffix': None, 'preserveAll': False})
                self.assertEqual(case['steps'][1], {
                    'op': 'resolve', 'packageInput': input_id, 'programId': program.attrib['Id'],
                    'expectStatus': 'unavailable' if input_id.endswith('oversized') else 'expanded',
                    'minimumSteps': 0 if input_id.endswith('oversized') else 5,
                    'requiredIssueCodes': ['source-byte-limit'] if input_id.endswith('oversized') else [],
                })
            self.assertEqual(len(seen_ids), 3)


    def test_output_requires_new_absolute_directory_outside_checkout(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            with chdir(root), self.assertRaisesRegex(ValueError, 'absolute'):
                generate(Path('relative'))
            self.assertFalse((root / 'relative').exists())
            checkout = Path(__file__).resolve().parents[2]
            with patch.object(Path, 'mkdir', side_effect=AssertionError('must refuse before mkdir')):
                with self.assertRaisesRegex(ValueError, 'checkout'):
                    generate(checkout / 'forbidden-measurement-fixtures')
                (root / 'checkout-alias').symlink_to(checkout, target_is_directory=True)
                with self.assertRaisesRegex(ValueError, 'checkout'):
                    generate(root / 'checkout-alias' / 'forbidden-measurement-fixtures')
            old = root / 'existing'
            old.mkdir()
            sentinel = old / 'sentinel'
            sentinel.write_bytes(b'unchanged')
            with self.assertRaises(FileExistsError):
                generate(old)
            self.assertEqual(sentinel.read_bytes(), b'unchanged')
            linked = root / 'linked'
            linked.symlink_to(old, target_is_directory=True)
            with self.assertRaises(FileExistsError):
                generate(linked)
            dangling = root / 'dangling'
            dangling.symlink_to(root / 'missing', target_is_directory=True)
            with self.assertRaises(FileExistsError):
                generate(dangling)
            self.assertFalse((root / 'missing').exists())


    def test_generation_is_deterministic_with_stored_crc_fixed_metadata_and_order(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        with TemporaryDirectory() as temporary:
            roots = [Path(temporary) / name for name in ('first', 'second')]
            manifests = [json.loads(generate(root).read_text(encoding='utf-8')) for root in roots]
            first, second = manifests
            self.assertNotEqual(first['inputRoot'], second['inputRoot'])
            first['inputRoot'] = second['inputRoot'] = '<owned-input-root>'
            self.assertEqual(first, second)
            self.assertEqual((roots[0] / 'fixture-recipe.json').read_bytes(),
                             (roots[1] / 'fixture-recipe.json').read_bytes())
            for item in first['inputs']:
                left = (roots[0] / item['path']).read_bytes()
                self.assertEqual(left, (roots[1] / item['path']).read_bytes())
                archives = [(item['id'], left)]
                if item['kind'] == 'wrapper':
                    with zipfile.ZipFile(io.BytesIO(left)) as outer:
                        archives.extend((name, outer.read(name)) for name in
                                        ['01-new.knxprod', '02-refused.knxprod'])
                for name, raw in archives:
                    with self.subTest(archive=name), zipfile.ZipFile(io.BytesIO(raw)) as archive:
                        self.assertIsNone(archive.testzip())
                        self.assertEqual(archive.comment, b'fictional-measurement-shared-b'
                                         if name == 'shared-b' else b'')
                        self.assertFalse(any(info.is_dir() for info in archive.infolist()))
                        for info in archive.infolist():
                            self.assertEqual(info.compress_type, zipfile.ZIP_STORED)
                            self.assertEqual(info.compress_size, info.file_size)
                            self.assertEqual(info.date_time, (2000, 1, 1, 0, 0, 0))
                            self.assertEqual(info.create_system, 3)
                            self.assertEqual(info.external_attr, 0o100600 << 16)
                            self.assertEqual(info.extra, b'')
                            self.assertEqual(info.comment, b'')
                            self.assertEqual(info.flag_bits, 0)
            for root in roots:
                self.assertEqual({path.name for path in root.iterdir()},
                                 {item['path'] for item in first['inputs']} |
                                 {'manifest.json', 'fixture-recipe.json'})
            existing = (roots[0] / 'manifest.json').read_bytes()
            with self.assertRaises(FileExistsError):
                generate(roots[0])
            self.assertEqual((roots[0] / 'manifest.json').read_bytes(), existing)
            # The root is part of Manifest v1. Also regenerate at the identical
            # owned pathname to pin byte-exact manifests, not only normalized JSON.
            expected_bytes = {path.name: path.read_bytes() for path in roots[0].iterdir()}
            shutil.rmtree(roots[0])
            generate(roots[0])
            self.assertEqual({path.name for path in roots[0].iterdir()}, set(expected_bytes))
            for path in roots[0].iterdir():
                self.assertTrue(path.read_bytes() == expected_bytes[path.name], path.name)

    def test_exact_manifest_inventory_cases_roles_and_program_identities(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        def install(input_id: str, preserve: bool = False) -> dict:
            return {'op': 'install', 'input': input_id, 'profile': 'standard', 'expectExit': 0,
                    'diagnosticSuffix': None, 'preserveAll': preserve}

        identities = {
            'ladder-1mib': '0001', 'ladder-4mib': '0004', 'ladder-16mib': '0010',
            'shared-a': '0100', 'shared-b': '0100', 'shared-alias': '0100',
            'evidence-distinct': '0101', 'resolver-small': '0301',
            'resolver-near-limit': '0302', 'resolver-oversized': '0303',
        }
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            self.assertEqual([item['id'] for item in manifest['inputs']], [
                'ladder-1mib', 'ladder-4mib', 'ladder-16mib', 'shared-a', 'shared-b',
                'shared-alias', 'evidence-distinct', 'rollback-wrapper', 'resolver-small',
                'resolver-near-limit', 'resolver-oversized'])
            expected_cases = [{'id': f'ladder-{mib}mib', 'steps': [install(f'ladder-{mib}mib')]}
                              for mib in (1, 4, 16)]
            expected_cases.extend([
                {'id': 'shared-retry-alias', 'steps': [install('shared-a'), install('shared-a', True),
                  install('shared-b'), install('shared-b', True), install('shared-alias')]},
                {'id': 'repeated-evidence', 'steps': [install('shared-a')]},
                {'id': 'distinct-evidence', 'steps': [install('evidence-distinct')]},
                {'id': 'seeded-rollback', 'steps': [install('shared-a'), {
                    'op': 'install', 'input': 'rollback-wrapper', 'profile': 'standard',
                    'expectExit': 1, 'preserveAll': True, 'diagnosticSuffix':
                    'inner product package 02-refused.knxprod: M-0001/late.xml: incomplete XML document'}]},
            ])
            for input_id in ['resolver-small', 'resolver-near-limit', 'resolver-oversized']:
                oversized = input_id == 'resolver-oversized'
                expected_cases.append({'id': input_id, 'steps': [install(input_id), {
                    'op': 'resolve', 'packageInput': input_id,
                    'programId': f'M-0001_A-{identities[input_id]}-01-0000',
                    'expectStatus': 'unavailable' if oversized else 'expanded',
                    'minimumSteps': 0 if oversized else 5,
                    'requiredIssueCodes': ['source-byte-limit'] if oversized else [],
                }]})
            self.assertEqual(manifest['cases'], expected_cases)
            for item in manifest['inputs']:
                if item['kind'] == 'wrapper':
                    self.assertEqual(item['path'], 'rollback-wrapper.zip')
                    continue
                program_id = f"M-0001_A-{identities[item['id']]}-01-0000"
                self.assertEqual(item['path'], f"{item['id']}.knxprod")
                members = item['retainedPackages'][0]['members']
                self.assertEqual([(m['ordinal'], m['path'], m['role']) for m in members], [
                    (0, 'knx_master.xml', 'Master'),
                    (1, f'M-0001/{program_id}.xml', 'ApplicationProgram'),
                    (2, 'M-0001/measurement.signature', 'Signature')])
                with zipfile.ZipFile(root / item['path']) as archive:
                    program = ET.fromstring(archive.read(members[1]['path']))
                    app = program.find('.//{http://knx.org/xml/project/20}ApplicationProgram')
                    self.assertEqual(app.attrib['Id'], program_id)
                    self.assertEqual(app.attrib['ApplicationNumber'], str(int(identities[item['id']], 16)))
                    self.assertEqual(app.attrib['ApplicationVersion'], '1')
                    self.assertEqual(archive.read(members[2]['path']), bytes(range(32)) +
                                     b'fictional-measurement-signature\x00\xff')

    def test_master_exercises_fixed_functions_space_and_translation_shape(self) -> None:
        from tools.productdb_measurement_fixtures import generate

        ns = {'k': 'http://knx.org/xml/project/20'}
        with TemporaryDirectory() as temporary:
            root = Path(temporary) / 'inputs'
            manifest = json.loads(generate(root).read_text(encoding='utf-8'))
            shared = next(item for item in manifest['inputs'] if item['id'] == 'shared-a')
            with zipfile.ZipFile(root / shared['path']) as archive:
                master_bytes = archive.read('knx_master.xml')
                master = ET.fromstring(master_bytes)
            recipe = json.loads((root / 'fixture-recipe.json').read_text(encoding='utf-8'))
            self.assertTrue('master' in recipe, 'missing fixed master preparation counts')
            self.assertEqual(recipe['master']['actualBytes'], len(master_bytes))
            self.assertEqual(recipe['master']['xml']['nodes'], len(list(master.iter())))
            self.assertEqual(recipe['master']['expectedConstructs'], {
                'manufacturers': 1, 'functionTypes': 1, 'functionPoints': 1,
                'spaceUsages': 1, 'translations': 2})
            self.assertEqual(master.find('k:MasterData/k:Manufacturers/k:Manufacturer', ns).attrib,
                             {'Id': 'M-0001', 'Name': 'Fictional Measurement'})
            function = master.find('k:MasterData/k:FunctionTypes/k:FunctionType', ns)
            self.assertEqual(function.attrib, {'Id': 'FT-1', 'Number': '1', 'Text': 'Switch', 'Status': 'Certified'})
            self.assertEqual(function[0].attrib, {'Id': 'FP-1_DR-1', 'Text': 'Switch',
                             'DatapointType': 'DPST-1-1', 'Role': 'Control', 'Characteristics': 'W'})
            self.assertEqual(master.find('k:MasterData/k:SpaceUsages/k:SpaceUsage', ns).attrib,
                             {'Id': 'SU-1', 'Number': '1', 'Text': 'Office'})
            language = master.find('k:Languages/k:Language', ns)
            self.assertEqual(language.attrib, {'Identifier': 'de-DE'})
            self.assertEqual([(unit.attrib['RefId'], unit[0].attrib['RefId'], unit[0][0].attrib)
                              for unit in language], [
                ('FT-1', 'FT-1', {'AttributeName': 'Text', 'Text': 'Schalten'}),
                ('SU-1', 'SU-1', {'AttributeName': 'Text', 'Text': 'Büro'}),
            ])


if __name__ == '__main__':
    unittest.main()
