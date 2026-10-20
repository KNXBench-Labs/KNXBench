"""Read-only SQLite endpoint accounting; not an installer or schema authority."""
from __future__ import annotations

from contextlib import contextmanager
import filecmp
import hashlib
import io
import json
from pathlib import Path
import sqlite3
import struct
import zipfile


# Opt-in study oracle ceilings, not production importer limits or an RSS claim.
# These admit the generated eleven-input study (~60 MiB decoded). Charge every
# admitted occurrence before deduplication; never trim bytes/history to fit.
SOURCE_ORACLE_MAX_INPUT_BYTES = 128 * 1024 * 1024
SOURCE_ORACLE_MAX_PACKAGE_BYTES = 128 * 1024 * 1024
SOURCE_ORACLE_MAX_DECODED_BYTES = 128 * 1024 * 1024
SOURCE_ORACLE_MAX_MEMBER_BYTES = 32 * 1024 * 1024
SOURCE_ORACLE_MAX_DB_RETAINED_BYTES = 256 * 1024 * 1024
SOURCE_ORACLE_MAX_RECORDS = 100_000


def quote(name):
    return '"' + name.replace('"', '""') + '"'


@contextmanager
def reader(path):
    path = Path(path)
    if not path.is_file():
        raise ValueError('missing database')
    with path.open('rb') as stream:
        header = stream.read(20)
    if header[:16] != b'SQLite format 3\0' or header[18:20] != b'\x01\x01':
        raise ValueError('unsupported SQLite file or journal header')
    # immutable avoids recovery or WAL/SHM creation by an observer. Layout
    # admission precedes reads; this is valid only for the owned quiescent DB.
    for suffix in ('-wal', '-shm', '-journal'):
        sidecar = Path(str(path) + suffix)
        if sidecar.exists() and (suffix != '-journal' or sidecar.stat().st_size):
            raise ValueError('unsupported database sidecar')
    db = sqlite3.connect(path.resolve().as_uri() + '?mode=ro&immutable=1', uri=True)
    try:
        db.execute('PRAGMA query_only=ON')
        # CAST(text AS BLOB) below measures UTF-8 logical bytes only in UTF-8
        # databases. Refuse other encodings before yielding any accounting API.
        if db.execute('PRAGMA encoding').fetchone()[0] != 'UTF-8':
            raise ValueError('unsupported database encoding; UTF-8 required')
        if db.execute('PRAGMA journal_mode').fetchone()[0].lower() != 'delete':
            raise ValueError('unsupported journal mode')
        if db.execute('PRAGMA auto_vacuum').fetchone()[0] != 0:
            raise ValueError('unsupported auto-vacuum layout')
        yield db
    finally:
        db.close()


def file_stat(path):
    try:
        stat = path.stat()
    except FileNotFoundError:
        return {'exists': False, 'sizeBytes': 0, 'allocatedBytes': 0}
    return {'exists': True, 'sizeBytes': stat.st_size, 'allocatedBytes': stat.st_blocks * 512}


def objects(db):
    return [('table', 'sqlite_schema', 'sqlite_schema', 1)] + list(db.execute(
        "SELECT type,name,tbl_name,rootpage FROM main.sqlite_schema "
        "WHERE type IN ('table','index') ORDER BY type,name"))


def snapshot(path):
    path = Path(path)
    with reader(path) as db:
        pragmas = {key: db.execute('PRAGMA ' + name).fetchone()[0] for key, name in (
            ('userVersion', 'user_version'), ('pageSize', 'page_size'), ('pageCount', 'page_count'),
            ('freelistCount', 'freelist_count'), ('journalMode', 'journal_mode'),
            ('autoVacuum', 'auto_vacuum'), ('encoding', 'encoding'))}
        try:
            stats = {row[0]: row[1:] for row in db.execute(
                "SELECT name,count(*),sum(pgsize),sum(payload),sum(unused),"
                "sum(pagetype='overflow') FROM dbstat WHERE aggregate=FALSE GROUP BY name")}
        except sqlite3.DatabaseError as error:
            raise ValueError('dbstat unavailable') from error
        inventory = objects(db)
        if set(stats) - {o[1] for o in inventory}:
            raise ValueError('unmapped dbstat tree')
        entries, roots = [], set()
        for kind, name, owner, root in inventory:
            own = root > 0 and root not in roots
            if own and name not in stats:
                raise ValueError('unattributed root page')
            roots.add(root)
            pages = stats.get(name, (0, 0, 0, 0, 0)) if own else (0, 0, 0, 0, 0)
            columns = []
            if kind == 'table':
                for col in db.execute('PRAGMA table_xinfo(' + quote(name) + ')'):
                    column = quote(col[1])
                    sizes = db.execute('SELECT ' + ','.join(
                        f"coalesce(sum(CASE WHEN typeof({column})='{t}' THEN "
                        f"length(CAST({column} AS BLOB)) ELSE 0 END),0)" for t in ('text', 'blob'))
                        + ' FROM ' + quote(name)).fetchone()
                    columns.append({'name': col[1], 'textBytes': sizes[0], 'blobBytes': sizes[1]})
            entries.append({'type': kind, 'name': name, 'ownerTable': owner, 'rootPage': root,
                            'rows': db.execute('SELECT count(*) FROM ' + quote(name)).fetchone()[0]
                            if kind == 'table' else None, 'logicalColumns': columns,
                            **dict(zip(('pages', 'bytes', 'payloadBytes', 'unusedBytes', 'overflowPages'), pages)),
                            'hasOwnBtree': own})
        files = {key: file_stat(Path(str(path) + suffix)) for key, suffix in
                 (('main', ''), ('wal', '-wal'), ('shm', '-shm'), ('journal', '-journal'))}
        btree = sum(o['pages'] for o in entries)
        residual = pragmas['pageCount'] - btree - pragmas['freelistCount']
        logical = pragmas['pageCount'] * pragmas['pageSize']
        slack = files['main']['sizeBytes'] - logical
        if residual or slack:
            raise ValueError('unreconciled database pages or file slack')
        return {'pragmas': pragmas, 'files': files, 'objects': entries,
                'reconciliation': {'btreePages': btree, 'schemaPages': stats['sqlite_schema'][0],
                                   'freePages': pragmas['freelistCount'], 'otherPages': residual,
                                   'otherReason': None, 'logicalDatabaseBytes': logical,
                                   'mainFileSlackBytes': slack},
                'retained': retained(db), 'counters': counters(db)}


def retained(db):
    tables = {row[1] for row in objects(db) if row[0] == 'table'}
    result = dict.fromkeys(('packageBlobBytes', 'sourceBlobBytes', 'memberReferenceBytes',
                           'distinctReferencedSourceCount', 'distinctReferencedSourceBytes',
                           'packageLengthMismatchCount', 'sourceLengthMismatchCount',
                           'memberLengthMismatchCount'), 0)
    for table, declared, total, mismatch in (
        ('package', 'size', 'packageBlobBytes', 'packageLengthMismatchCount'),
        ('source_file', 'len', 'sourceBlobBytes', 'sourceLengthMismatchCount')):
        if table in tables:
            result[total], result[mismatch] = db.execute(
                f'SELECT coalesce(sum(length(bytes)),0),coalesce(sum({quote(declared)} != length(bytes)),0) '
                f'FROM {quote(table)}').fetchone()
    if {'source_file', 'package_member'} <= tables:
        result['memberReferenceBytes'] = db.execute(
            'SELECT coalesce(sum(size),0) FROM package_member').fetchone()[0]
        result['distinctReferencedSourceCount'], result['distinctReferencedSourceBytes'] = db.execute(
            'SELECT count(*),coalesce(sum(length(bytes)),0) FROM source_file '
            'WHERE sha256 IN (SELECT source_sha256 FROM package_member)').fetchone()
        result['memberLengthMismatchCount'] = db.execute(
            'SELECT count(*) FROM package_member m LEFT JOIN source_file s ON s.sha256=m.source_sha256 '
            'WHERE s.sha256 IS NULL OR m.size != length(s.bytes)').fetchone()[0]
    return result


def counters(db):
    # Units follow the owning schema, never a guessed sum of integer columns.
    definitions = [
        ('ingest_unknown', 'occurrences', ('kind',), 'source-unknown-occurrences'),
        ('package_install_unknown', 'occurrences', ('kind',), 'package-unknown-occurrences'),
        ('package_install_report', 'unknown_distinct', ('status',), 'package-unknown-distinct'),
        ('package_install_report', 'unknown_occurrences', ('status',), 'package-unknown-occurrences'),
        ('package_install_count', 'count', ('category', 'disposition'), 'category-entities'),
        ('package_install_diagnostic', 'occurrences', ('kind',), 'diagnostic-occurrences'),
        ('package_conflict', 'occurrence', ('table_name',), 'per-parse-conflict-repeats')]
    table_columns = {name: {row[1] for row in db.execute('PRAGMA table_xinfo(' + quote(name) + ')')}
                     for kind, name, _, _ in objects(db) if kind == 'table'}
    for column in ('unknown_count', 'translation_program_count', 'translation_catalog_count',
                   'translation_hardware_count', 'translation_master_count', 'dropped_datapoint_type_count'):
        if column in table_columns.get('package', set()):
            definitions.append(('package', column, (), 'stored-' + column.replace('_', '-')))
    result = []
    for table, column, group, unit in definitions:
        if table not in table_columns:
            continue
        if column not in table_columns[table]:
            raise ValueError('missing defined evidence counter')
        select = ','.join([*(quote(c) for c in group), 'coalesce(sum(' + quote(column) + '),0)'])
        sql = 'SELECT ' + select + ' FROM ' + quote(table)
        if group:
            sql += ' GROUP BY ' + ','.join(quote(c) for c in group) + ' ORDER BY ' + ','.join(quote(c) for c in group)
        for row in db.execute(sql):
            result.append({'table': table, 'column': column, 'group': dict(zip(group, row[:-1])),
                           'value': None if table == 'package_install_report' and row[0] == 'unavailable'
                           else row[-1], 'unit': unit})
    return result


def scalar(value):
    if value is None:
        tag, data = b'N', b''
    elif type(value) is int:
        tag, data = b'I', str(value).encode('ascii')
    elif type(value) is float:
        tag, data = b'F', struct.pack('>d', value)
    elif type(value) is str:
        tag, data = b'T', value.encode('utf-8')
    elif type(value) is bytes:
        tag, data = b'B', value
    else:
        raise ValueError('unsupported SQLite scalar')
    return tag + struct.pack('>Q', len(data)) + data


def logical_snapshot(path, output, exclude=()):
    """Persist complete typed row streams, including row identity and schema SQL."""
    output = Path(output)
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    result = {}
    with reader(path) as db:
        inventory = list(db.execute('SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema '
                                    'ORDER BY type,name'))
        with (output / 'schema.rows').open('xb') as stream:
            for row in inventory:
                for value in row:
                    stream.write(scalar(value))
        result['__schema__'] = output / 'schema.rows'
        for index, (_, name, _, _) in enumerate(objects(db)):
            if name in exclude or name in result:
                continue
            kind = next(o[0] for o in objects(db) if o[1] == name)
            if kind != 'table':
                continue
            columns = list(db.execute('PRAGMA table_xinfo(' + quote(name) + ')'))
            # table_list's wr flag distinguishes WITHOUT ROWID without guessing
            # from PK declarations or SQL text (which can contain that phrase).
            layouts = {row[1]: row[4] for row in db.execute('PRAGMA main.table_list')}
            if name not in layouts:
                raise ValueError('unavailable table row identity layout')
            selection = '*'
            if layouts[name]:
                pk = [c[1] for c in sorted(columns, key=lambda c: c[5]) if c[5]]
                if not pk:
                    raise ValueError('missing WITHOUT ROWID primary key')
                order = ','.join(quote(c) for c in pk)
            else:
                # A declared TEXT/composite PK does not replace hidden rowid.
                # Intentionally refuse all-shadowed aliases, even with a PK:
                # do not claim complete SQL-visible identity from partial rows.
                column_names = {c[1].lower() for c in columns}
                rowid = next((c for c in ('rowid', '_rowid_', 'oid') if c not in column_names), None)
                if rowid is None:
                    raise ValueError('no unshadowed row identity')
                selection = rowid + ',*'
                order = rowid
            target = output / f'{index:04d}.rows'
            with target.open('xb') as stream:
                for col in columns:
                    stream.write(scalar(col[1]))
                for row in db.execute('SELECT ' + selection + ' FROM ' + quote(name) + ' ORDER BY ' + order):
                    stream.write(b'R' + struct.pack('>Q', len(row)))
                    for value in row:
                        stream.write(scalar(value))
            result[name] = target
    return result


def streams_equal(before, after, exclude=()):
    left = {k: v for k, v in before.items() if k not in exclude}
    right = {k: v for k, v in after.items() if k not in exclude}
    return left.keys() == right.keys() and all(filecmp.cmp(left[k], right[k], shallow=False) for k in left)


def preservation_exact(path, before, after, required, alias_names=None, alias=None):
    if required:
        return streams_equal(before, after)
    if alias_names is None:
        return None
    with reader(path) as db:
        names = set(db.execute('SELECT package_sha256,source_name FROM package_source_name'))
    return (alias not in alias_names and names == alias_names | {alias} and
            streams_equal(before, after, exclude=('package_source_name',)))


def _oracle_limit(value, limit, unit):
    if type(value) is not int or value < 0 or value > limit:
        raise ValueError('source oracle ' + unit + ' limit exceeded')


def _source_oracle_preflight(db, admitted):
    """Reject oversized inventories before reading any external/DB payloads."""
    bounded = []
    input_bytes = decoded_bytes = records = 0
    for entry, source in admitted:
        records += 1
        _oracle_limit(records, SOURCE_ORACLE_MAX_RECORDS, 'record')
        input_bytes += Path(source).stat().st_size
        _oracle_limit(input_bytes, SOURCE_ORACLE_MAX_INPUT_BYTES, 'input byte')
        for package in entry['retainedPackages']:
            records += 1
            _oracle_limit(records, SOURCE_ORACLE_MAX_RECORDS, 'record')
            for member in package['members']:
                records += 1
                _oracle_limit(records, SOURCE_ORACLE_MAX_RECORDS, 'record')
                size = member['sizeBytes']
                _oracle_limit(size, SOURCE_ORACLE_MAX_MEMBER_BYTES, 'member byte')
                decoded_bytes += size
                _oracle_limit(decoded_bytes, SOURCE_ORACLE_MAX_DECODED_BYTES, 'decoded byte')
        bounded.append((entry, source))
    retained_bytes = db_records = 0
    for table, columns in (
            ('package', ('sha256', 'size', 'bytes')),
            ('source_file', ('sha256', 'source_path', 'manufacturer_id', 'len', 'bytes')),
            ('package_member', ('package_sha256', 'ordinal', 'path', 'role', 'source_sha256', 'size'))):
        # Charge actual values, including all returned metadata and unexpected
        # rows; stored size/len fields and reference membership are not bounds.
        lengths = '+'.join('coalesce(length(CAST(' + quote(c) + ' AS BLOB)),0)' for c in columns)
        count, size = db.execute('SELECT count(*),coalesce(sum(' + lengths + '),0) FROM ' + quote(table)).fetchone()
        db_records += count
        retained_bytes += size
        _oracle_limit(db_records, SOURCE_ORACLE_MAX_RECORDS, 'record')
        _oracle_limit(retained_bytes, SOURCE_ORACLE_MAX_DB_RETAINED_BYTES, 'database retained byte')
    return bounded


def _oracle_read(stream, limit, unit):
    data = stream.read(limit + 1)
    _oracle_limit(len(data), limit, unit)
    return data


def _oracle_zip_read(archive, info, limit, unit):
    _oracle_limit(info.file_size, limit, unit)
    with archive.open(info) as stream:
        return _oracle_read(stream, info.file_size, unit)


def verify_sources(path, admitted):
    """Exact external ZIP oracle for explicitly size-bounded opt-in studies.

    Preflight aggregate raw/declared decoded inputs and actual retained SQL
    values before mass loading, then enforce actual ZIP sizes before expansion.
    All budgets reset on each call; exceeding one refuses the whole observation.
    This does not measure target RSS or guarantee a constant observer RSS.
    """
    with reader(path) as db:
        admitted = _source_oracle_preflight(db, admitted)
    packages, sources, members = {}, {}, {}
    input_bytes = package_bytes = decoded_bytes = 0
    for entry, source in admitted:
        # A growing file cannot bypass the stat preflight with an unbounded read.
        with Path(source).open('rb') as stream:
            raw = _oracle_read(stream, SOURCE_ORACLE_MAX_INPUT_BYTES - input_bytes, 'input byte')
        input_bytes += len(raw)
        if hashlib.sha256(raw).hexdigest() != entry['sha256']:
            raise ValueError('external input changed')
        for oracle in entry['retainedPackages']:
            if oracle['innerPath'] is None:
                content = raw
            else:
                with zipfile.ZipFile(io.BytesIO(raw)) as outer:
                    matching = [i for i in outer.infolist() if i.filename == oracle['innerPath']]
                    if len(matching) != 1:
                        raise ValueError('ambiguous or missing expected inner package')
                    content = _oracle_zip_read(outer, matching[0],
                        SOURCE_ORACLE_MAX_PACKAGE_BYTES - package_bytes, 'package byte')
            package_bytes += len(content)
            _oracle_limit(package_bytes, SOURCE_ORACLE_MAX_PACKAGE_BYTES, 'package byte')
            package_sha = hashlib.sha256(content).hexdigest()
            if package_sha != oracle['sha256']:
                raise ValueError('external package authentication failed')
            packages[package_sha] = (len(content), content)
            expected_members = []
            with zipfile.ZipFile(io.BytesIO(content)) as archive:
                entries = [i for i in archive.infolist() if not i.is_dir()]
                if len(entries) != len(oracle['members']):
                    raise ValueError('external member inventory mismatch')
                for ordinal, (info, member) in enumerate(zip(entries, oracle['members'])):
                    _oracle_limit(info.file_size, SOURCE_ORACLE_MAX_MEMBER_BYTES, 'member byte')
                    data = _oracle_zip_read(archive, info,
                        SOURCE_ORACLE_MAX_DECODED_BYTES - decoded_bytes, 'decoded byte')
                    decoded_bytes += len(data)
                    digest = hashlib.sha256(data).hexdigest()
                    if (ordinal, info.filename, digest, len(data)) != (
                            member['ordinal'], member['path'], member['sha256'], member['sizeBytes']):
                        raise ValueError('external member authentication failed')
                    manufacturer = info.filename.split('/')[0]
                    manufacturer = manufacturer if manufacturer.startswith('M-') else None
                    sources.setdefault(digest, (info.filename, manufacturer, len(data), data))
                    expected_members.append((ordinal, info.filename, member['role'], digest, len(data)))
            if package_sha in members and members[package_sha] != expected_members:
                raise ValueError('conflicting package member oracle')
            members[package_sha] = expected_members
    with reader(path) as db:
        # Recheck the endpoint immediately before loading database payloads.
        _source_oracle_preflight(db, ())
        actual_packages = {sha: (size, blob) for sha, size, blob in db.execute(
            'SELECT sha256,size,bytes FROM package')}
        actual_sources = {sha: (source, manufacturer, size, blob) for sha, source, manufacturer, size, blob
                          in db.execute('SELECT sha256,source_path,manufacturer_id,len,bytes FROM source_file')}
        actual_members = {}
        for package_sha, ordinal, name, role, digest, size in db.execute(
                'SELECT package_sha256,ordinal,path,role,source_sha256,size FROM package_member '
                'ORDER BY package_sha256,ordinal'):
            actual_members.setdefault(package_sha, []).append((ordinal, name, role, digest, size))
    return {'packagesExact': actual_packages == packages,
            'sourcesExact': actual_sources == sources, 'memberIndexExact': actual_members == members}


def integrity(path):
    with reader(path) as db:
        return {'sqliteOk': list(db.execute('PRAGMA integrity_check')) == [('ok',)],
                'foreignKeysOk': not list(db.execute('PRAGMA foreign_key_check'))}


def delta(before, after):
    def subtract(left, right):
        result = {}
        for key in left.keys() | right.keys():
            a, b = left.get(key, 0), right.get(key, 0)
            if key == 'exists':
                result[key] = {'before': a, 'after': b}
            elif key == 'value' and (a is None or b is None):
                result[key] = None
            elif type(a) in (int, float) and type(b) in (int, float):
                result[key] = b - a
            elif type(a) is dict and type(b) is dict:
                result[key] = subtract(a, b)
            elif a == b:
                result[key] = a
            else:
                result[key] = {'before': a, 'after': b}
        return result
    def collection(left, right, identity):
        a, b = {identity(x): x for x in left}, {identity(x): x for x in right}
        changes = []
        for key in sorted(a.keys() | b.keys()):
            change = dict(subtract(a.get(key, {}), b.get(key, {})),
                          added=key not in a, removed=key not in b)
            current = b.get(key, a.get(key))
            for field in ('type', 'name', 'ownerTable', 'table', 'column', 'group', 'unit'):
                if field in current:
                    change[field] = current[field]
            if current.get('type') == 'index':
                change['rows'] = None
            changes.append(change)
        return changes
    for key in ('userVersion', 'pageSize', 'journalMode', 'autoVacuum', 'encoding'):
        if before['pragmas'][key] != after['pragmas'][key]:
            raise ValueError('nonadditive database property changed')
    result = {key: subtract(before[key], after[key]) for key in
              ('pragmas', 'files', 'reconciliation', 'retained')}
    result['objects'] = collection(before['objects'], after['objects'], lambda o: (o['type'], o['name']))
    for obj in result['objects']:
        a = next((o for o in before['objects'] if o['name'] == obj['name'] and o['type'] == obj['type']), {})
        b = next((o for o in after['objects'] if o['name'] == obj['name'] and o['type'] == obj['type']), {})
        obj['logicalColumns'] = collection(a.get('logicalColumns', []), b.get('logicalColumns', []),
                                           lambda c: c['name'])
    result['counters'] = collection(before['counters'], after['counters'],
                                   lambda c: (c['table'], c['column'], json.dumps(c['group'], sort_keys=True)))
    return result
