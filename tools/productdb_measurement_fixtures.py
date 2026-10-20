"""Deterministic synthetic inputs for the opt-in ProductDB measurement tool.

Only source fixtures and external byte/member oracles are generated here. These
shapes follow existing repository tests; generation is not importer acceptance.
"""
from __future__ import annotations

import hashlib
import io
import json
from pathlib import Path
from xml.parsers import expat
import zipfile


_NS = 'http://knx.org/xml/project/20'
_MAX_XML_BYTES = 8 * 1024 * 1024
_SEQUENCE = (
    '<LdCtrlConnect/><LdCtrlMerge MergeId="1"/><LdCtrlLoad LsmIdx="4"/>'
    '<LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/>'
    '<LdCtrlMerge MergeId="6"/><LdCtrlMerge MergeId="7"/><LdCtrlRestart/>'
)
_FRAGMENTS = (
    '<LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16"/></LoadProcedure>'
    '<LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Size="16"/></LoadProcedure>'
)
_SHARED_ID = 'M-0001_A-0100-01-0000'
_SIGNATURE = bytes(range(32)) + b'fictional-measurement-signature\x00\xff'
_STAMP = (2000, 1, 1, 0, 0, 0)


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _archive(entries: list[tuple[str, bytes]], comment: bytes = b'') -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', compression=zipfile.ZIP_STORED) as archive:
        archive.comment = comment
        for path, data in entries:
            info = zipfile.ZipInfo(path, date_time=_STAMP)
            info.create_system = 3
            info.external_attr = 0o100600 << 16
            archive.writestr(info, data)
    return buffer.getvalue()


def _master() -> bytes:
    return (
        f'<KNX xmlns="{_NS}"><MasterData><Manufacturers>'
        '<Manufacturer Id="M-0001" Name="Fictional Measurement"/>'
        '</Manufacturers><FunctionTypes>'
        '<FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified">'
        '<FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" '
        'Role="Control" Characteristics="W"/></FunctionType></FunctionTypes>'
        '<SpaceUsages><SpaceUsage Id="SU-1" Number="1" Text="Office"/></SpaceUsages>'
        '<MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures>'
        '<Procedure ProcedureType="Load" ProcedureSubType="ap1" Access="remote local2">'
        f'{_SEQUENCE}</Procedure></Procedures></HawkConfigurationData></MaskVersion>'
        '</MaskVersions></MasterData><Languages><Language Identifier="de-DE">'
        '<TranslationUnit RefId="FT-1"><TranslationElement RefId="FT-1">'
        '<Translation AttributeName="Text" Text="Schalten"/>'
        '</TranslationElement></TranslationUnit>'
        '<TranslationUnit RefId="SU-1"><TranslationElement RefId="SU-1">'
        '<Translation AttributeName="Text" Text="Büro"/>'
        '</TranslationElement></TranslationUnit></Language></Languages></KNX>'
    ).encode('utf-8')


def _xml_metrics(data: bytes) -> dict:
    """Count generated-subset structure, not production parser admission/work."""
    nodes = depth = max_depth = max_attributes = 0
    parser = expat.ParserCreate()

    def start(_name: str, attrs: dict) -> None:
        nonlocal nodes, depth, max_depth, max_attributes
        nodes += 1
        depth += 1
        max_depth = max(max_depth, depth)
        max_attributes = max(max_attributes, len(attrs))

    def end(_name: str) -> None:
        nonlocal depth
        depth -= 1

    parser.StartElementHandler = start
    parser.EndElementHandler = end
    parser.Parse(data, True)
    # Generated XML has only ordinary tags and safe attribute text: no literal
    # '<' in values, free text, comments, DTD, processing instructions or CDATA.
    return {'nodes': nodes, 'eventsIncludingEof': data.count(b'<') + 1,
            'maxDepth': max_depth, 'maxRawAttributes': max_attributes}


def _program(program_id: str, rows: int = 16, *, target: int | None = None,
             distinct: bool = False, resolver: bool = False,
             exact_size: int | None = None) -> tuple[bytes, dict]:
    application_number = int(program_id.split('_A-', 1)[1].split('-', 1)[0], 16)
    head = (
        f'<KNX xmlns="{_NS}"><ManufacturerData><Manufacturer RefId="M-0001">'
        f'<ApplicationPrograms><ApplicationProgram Id="{program_id}" '
        f'ApplicationNumber="{application_number}" ApplicationVersion="1" ProgramType="ApplicationProgram" '
        'MaskVersion="MV-07B0" Name="Fictional Measurement" DefaultLanguage="en-US" '
        'LoadProcedureStyle="MergedProcedure" PeiType="0" DynamicTableManagement="false" '
        'Linkable="false"><Static><ParameterTypes>'
        f'<ParameterType Id="{program_id}_PT-N" Name="N">'
        '<TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/>'
        '</ParameterType></ParameterTypes><Parameters>'
    )
    procedures = f'<LoadProcedures>{_FRAGMENTS}</LoadProcedures>' if resolver else ''
    tail = (f'</Parameters>{procedures}</Static></ApplicationProgram>'
            '</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>').encode('utf-8')
    text = 'X' * 512 + '-λ'
    out = bytearray(head.encode('utf-8'))
    count = 0
    last_row = b''
    while ((target is not None and len(out) + len(tail) < target)
           or (target is None and count < rows)):
        key = f'FutureMeasure{count:02d}' if distinct else 'FutureMeasure'
        last_row = (
            f'<Parameter Id="{program_id}_P-{count}" Name="P{count}" '
            f'ParameterType="{program_id}_PT-N" Text="{text}" Value="0" '
            f'{key}="first-λ"/>'
        ).encode('utf-8')
        out.extend(last_row)
        count += 1
    extra = 0
    if exact_size is not None:
        extra = exact_size - len(out) - len(tail)
        if extra < 0 or not last_row:
            raise ValueError('exact target must fit the complete fixed-row program')
        position = last_row.index(b'" Value="0"')
        extended = last_row[:position] + b'Y' * extra + last_row[position:]
        out[-len(last_row):] = extended
        last_row = extended
    out.extend(tail)
    data = bytes(out)
    return data, {
        'programId': program_id, 'memberPath': f'M-0001/{program_id}.xml',
        'targetBytes': exact_size if exact_size is not None else target,
        'actualBytes': len(data), 'parameterRows': count,
        'lastRowBytes': len(last_row), 'textUtf8Bytes': count * len(text.encode('utf-8')) + extra,
        'unknownSubset': ([{'name': f'FutureMeasure{index:02d}', 'occurrences': 1,
                            'sample': 'first-λ'} for index in range(count)] if distinct else
                          [{'name': 'FutureMeasure', 'occurrences': count, 'sample': 'first-λ'}]),
        'sizing': ('final-persisted-text' if exact_size is not None else
                   'first-complete-row-at-target' if target is not None else 'fixed-rows'),
        'xml': _xml_metrics(data),
    }


def _entries(program_id: str, program: bytes) -> list[tuple[str, bytes]]:
    return [
        ('knx_master.xml', _master()),
        (f'M-0001/{program_id}.xml', program),
        ('M-0001/measurement.signature', _SIGNATURE),
    ]


def _package(program_id: str, program: bytes, comment: bytes = b'') -> tuple[bytes, dict]:
    entries = _entries(program_id, program)
    raw = _archive(entries, comment)
    oracle = {'innerPath': None, 'sha256': _sha(raw), 'members': [
        {'ordinal': ordinal, 'path': path, 'role': role,
         'sha256': _sha(data), 'sizeBytes': len(data)}
        for ordinal, ((path, data), role) in enumerate(zip(
            entries, ['Master', 'ApplicationProgram', 'Signature'], strict=True))
    ]}
    return raw, oracle


def _install(input_id: str, *, preserve: bool = False) -> dict:
    return {'op': 'install', 'input': input_id, 'profile': 'standard',
            'expectExit': 0, 'diagnosticSuffix': None, 'preserveAll': preserve}


def _write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def _save_package(output: Path, input_id: str, program_id: str, data: bytes,
                  comment: bytes = b'') -> dict:
    raw, oracle = _package(program_id, data, comment)
    path = f'{input_id}.knxprod'
    (output / path).write_bytes(raw)
    return {'id': input_id, 'path': path, 'sha256': _sha(raw),
            'kind': 'package', 'retainedPackages': [oracle]}


def generate(output: Path) -> Path:
    """Create a new private directory outside this checkout; never overwrite.

    Inputs are fictional. The separate recipe describes source-derived expected
    facts, not verified normalized rows or successful resolver admission.
    """
    if not output.is_absolute():
        raise ValueError('output must be an absolute path')
    if output.exists() or output.is_symlink():
        raise FileExistsError('output must not exist')
    checkout = Path(__file__).resolve().parents[1]
    canonical = output.resolve()
    if canonical.is_relative_to(checkout) or checkout.is_relative_to(canonical):
        raise ValueError('output must not overlap the checkout')
    output.mkdir(mode=0o700)
    inputs = []
    cases = []
    programs = {}
    for mib in (1, 4, 16):
        input_id = f'ladder-{mib}mib'
        program_id = f'M-0001_A-{mib:04X}-01-0000'
        data, recipe = _program(program_id, target=mib * 1024 * 1024)
        inputs.append(_save_package(output, input_id, program_id, data))
        programs[input_id] = recipe
        cases.append({'id': input_id, 'steps': [_install(input_id)]})
    data, recipe = _program(_SHARED_ID)
    inputs.append(_save_package(output, 'shared-a', _SHARED_ID, data))
    programs['shared-a'] = recipe
    inputs.append(_save_package(output, 'shared-b', _SHARED_ID, data,
                                b'fictional-measurement-shared-b'))
    inputs.append(_save_package(output, 'shared-alias', _SHARED_ID, data))
    programs['shared-b'] = recipe
    programs['shared-alias'] = recipe
    cases.append({'id': 'shared-retry-alias', 'steps': [
        _install('shared-a'), _install('shared-a', preserve=True),
        _install('shared-b'), _install('shared-b', preserve=True),
        _install('shared-alias'),
    ]})
    cases.append({'id': 'repeated-evidence', 'steps': [_install('shared-a')]})
    distinct_id = 'M-0001_A-0101-01-0000'
    data, recipe = _program(distinct_id, distinct=True)
    inputs.append(_save_package(output, 'evidence-distinct', distinct_id, data))
    programs['evidence-distinct'] = recipe
    cases.append({'id': 'distinct-evidence', 'steps': [_install('evidence-distinct')]})
    new_id = 'M-0001_A-0201-01-0000'
    refused_id = 'M-0001_A-0202-01-0000'
    first_program, first_recipe = _program(new_id)
    second_program, second_recipe = _program(refused_id)
    first_raw, _ = _package(new_id, first_program)
    second_raw = _archive(_entries(refused_id, second_program) + [('M-0001/late.xml', b'<Broken>')])
    wrapper = _archive([('01-new.knxprod', first_raw), ('02-refused.knxprod', second_raw),
                        ('Manual.txt', b'Fictional unretained wrapper manual.\n')])
    (output / 'rollback-wrapper.zip').write_bytes(wrapper)
    inputs.append({'id': 'rollback-wrapper', 'path': 'rollback-wrapper.zip',
                   'sha256': _sha(wrapper), 'kind': 'wrapper', 'retainedPackages': []})
    suffix = ('inner product package 02-refused.knxprod: '
              'M-0001/late.xml: incomplete XML document')
    cases.append({'id': 'seeded-rollback', 'steps': [
        _install('shared-a'), {'op': 'install', 'input': 'rollback-wrapper',
                              'profile': 'standard', 'expectExit': 1,
                              'diagnosticSuffix': suffix, 'preserveAll': True},
    ]})
    for index, (input_id, size) in enumerate([
        ('resolver-small', None), ('resolver-near-limit', _MAX_XML_BYTES - 4096),
        ('resolver-oversized', _MAX_XML_BYTES + 4096),
    ], start=0x301):
        program_id = f'M-0001_A-{index:04X}-01-0000'
        data, recipe = _program(program_id, resolver=True, exact_size=size)
        inputs.append(_save_package(output, input_id, program_id, data))
        programs[input_id] = recipe
        oversized = input_id == 'resolver-oversized'
        cases.append({'id': input_id, 'steps': [
            _install(input_id), {'op': 'resolve', 'packageInput': input_id,
                                 'programId': program_id,
                                 'expectStatus': 'unavailable' if oversized else 'expanded',
                                 'minimumSteps': 0 if oversized else 5,
                                 'requiredIssueCodes': ['source-byte-limit'] if oversized else []},
        ]})
    manifest = {'format': 1, 'inputRoot': str(output.resolve()),
                'inputs': inputs, 'cases': cases}
    _write_json(output / 'fixture-recipe.json', {
        'format': 1, 'acceptance': 'source-shaped-only; importer and resolver acceptance pending',
        'xmlMetricsMethod': 'generated-subset lexical tags plus EOF; Expat structural counts',
        'resolverSourceByteLimit': _MAX_XML_BYTES,
        'master': {'memberPath': 'knx_master.xml', 'actualBytes': len(_master()),
                   'xml': _xml_metrics(_master()), 'expectedConstructs': {
                       'manufacturers': 1, 'functionTypes': 1, 'functionPoints': 1,
                       'spaceUsages': 1, 'translations': 2}},
        'programs': programs,
        'rollback': {'seedInput': 'shared-a', 'innerPrograms': [first_recipe, second_recipe],
                     'innerPaths': ['01-new.knxprod', '02-refused.knxprod'],
                     'expectedDiagnosticSuffix': suffix},
    })
    manifest_path = output / 'manifest.json'
    _write_json(manifest_path, manifest)
    return manifest_path
