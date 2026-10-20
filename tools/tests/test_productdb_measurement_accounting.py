"""Read-only accounting controls; SQL builds only observer self-test databases."""
from __future__ import annotations

from contextlib import closing, contextmanager
import importlib
import importlib.util
import hashlib
import io
import zipfile
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
from unittest.mock import patch
import unittest


def accounting(test):
    test.assertIsNotNone(importlib.util.find_spec('tools.productdb_measurement_accounting'),
                         'WP04.1 all-object accounting API is missing')
    return importlib.import_module('tools.productdb_measurement_accounting')


class AccountingTests(unittest.TestCase):
    def test_inventory_dbstat_text_and_zero_deltas_cover_every_object(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('''CREATE TABLE "quoted λ" (id TEXT PRIMARY KEY,
                    value TEXT UNIQUE, payload BLOB) STRICT;
                    CREATE TABLE untouched (id INTEGER PRIMARY KEY) STRICT;''')
                db.execute('INSERT INTO "quoted λ" VALUES (?,?,?)',
                           ('id', 'λ\0漢' + 'x' * 9000, b'b' * 15000))
                raw = {row[0]: row[1:] for row in db.execute('''SELECT name,count(*),sum(pgsize),
                    sum(payload),sum(unused),sum(pagetype='overflow') FROM dbstat GROUP BY name''')}
                names = {('table', 'sqlite_schema'), *db.execute(
                    "SELECT type,name FROM sqlite_schema WHERE type IN ('table','index')")}
            snapshot = api.snapshot(path)
            self.assertEqual({(o['type'], o['name']) for o in snapshot['objects']}, names)
            for obj in snapshot['objects']:
                self.assertEqual(tuple(obj[key] for key in
                    ('pages', 'bytes', 'payloadBytes', 'unusedBytes', 'overflowPages')), raw[obj['name']])
                if obj['type'] == 'index':
                    self.assertIsNone(obj['rows'])
                    self.assertEqual(obj['ownerTable'], 'quoted λ')
            table = next(o for o in snapshot['objects'] if o['name'] == 'quoted λ')
            columns = {c['name']: c for c in table['logicalColumns']}
            self.assertEqual(columns['value']['textBytes'], len(('λ\0漢' + 'x' * 9000).encode()))
            self.assertEqual(columns['payload']['blobBytes'], 15000)
            reconciliation = snapshot['reconciliation']
            self.assertEqual(reconciliation['btreePages'], sum(o['pages'] for o in snapshot['objects']))
            self.assertEqual(reconciliation['otherPages'], 0)
            self.assertEqual(reconciliation['mainFileSlackBytes'], 0)
            delta = api.delta(snapshot, snapshot)
            self.assertEqual(len(delta['objects']), len(snapshot['objects']))
            self.assertTrue(all(o['pages'] == 0 and not o['added'] and not o['removed']
                                for o in delta['objects']))


    def test_delta_preserves_added_removed_identities_and_unavailable_values(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE old_table (id INT)')
            before = api.snapshot(path)
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('DROP TABLE old_table;CREATE TABLE new_table (id INT)')
            after = api.snapshot(path)
            change = api.delta(before, after)
            self.assertTrue(all(type(o['name']) is str for o in change['objects']),
                            'delta must preserve added/removed object identities')
            by_name = {o['name']: o for o in change['objects']}
            self.assertTrue(by_name['old_table']['removed'])
            self.assertTrue(by_name['new_table']['added'])
            self.assertEqual(by_name['new_table']['logicalColumns'][0]['name'], 'id')

    def test_evidence_units_unavailable_status_and_defined_conflict_counts(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('''CREATE TABLE ingest_unknown (kind TEXT,occurrences INT,sample TEXT);
                    INSERT INTO ingest_unknown VALUES ('Attribute',16,'first-λ');
                    CREATE TABLE package_install_report (status TEXT,unknown_distinct INT,unknown_occurrences INT);
                    INSERT INTO package_install_report VALUES ('measured',1,16),('unavailable',0,0);
                    CREATE TABLE package_conflict (table_name TEXT,occurrence INT);
                    INSERT INTO package_conflict VALUES ('parameter',3);
                    CREATE TABLE source_identity (occurrence INT);INSERT INTO source_identity VALUES (7);
                    CREATE TABLE package (unknown_count INT,translation_program_count INT,dropped_datapoint_type_count INT,
                        translation_future_key INT,size INT,bytes BLOB);
                    INSERT INTO package VALUES (1,2,3,999,0,X'');''')
            counters = api.snapshot(path)['counters']
            get = lambda table, column, group: next(c for c in counters if
                (c['table'],c['column'],c['group']) == (table,column,group))
            self.assertEqual(get('ingest_unknown','occurrences',{'kind':'Attribute'})['value'],16)
            self.assertIsNone(get('package_install_report','unknown_occurrences',{'status':'unavailable'})['value'])
            self.assertTrue(any(c['table'] == 'package_conflict' for c in counters),
                            'defined conflict repeat counter is missing')
            self.assertEqual(get('package_conflict','occurrence',{'table_name':'parameter'})['value'],3)
            self.assertEqual(get('package','dropped_datapoint_type_count',{})['value'],3)
            self.assertFalse(any(c['table'] == 'source_identity' for c in counters))
            self.assertFalse(any(c['column'] == 'translation_future_key' for c in counters),
                             'unknown integer columns must not become guessed semantic counters')
            delta = api.delta(api.snapshot(path), api.snapshot(path))
            self.assertIsNone(next(c for c in delta['counters'] if c['group'] == {'status':'unavailable'})['value'])

    def test_added_unavailable_counter_is_not_a_zero_and_file_existence_is_paired(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE package_install_report (status TEXT,unknown_distinct INT,unknown_occurrences INT)')
            before = api.snapshot(path)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("INSERT INTO package_install_report VALUES ('unavailable',0,0)")
            change = api.delta(before,api.snapshot(path))
            self.assertTrue(all(c['value'] is None for c in change['counters']),
                            'unavailable historical counters remain unavailable in deltas')
            self.assertEqual(change['files']['main']['exists'],{'before':True,'after':True})

    def test_freelist_and_reference_multiplicity_have_separate_totals(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('''CREATE TABLE grow (data BLOB);
                    CREATE TABLE source_file (sha256 TEXT,len INT,bytes BLOB);
                    INSERT INTO source_file VALUES ('s',4,X'01020304');
                    CREATE TABLE package_member (source_sha256 TEXT,size INT);
                    INSERT INTO package_member VALUES ('s',4),('s',4);''')
                db.execute('INSERT INTO grow VALUES (?)',(b'x'*150000,))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('DELETE FROM grow')
            snapshot = api.snapshot(path)
            self.assertGreater(snapshot['pragmas']['freelistCount'],0)
            self.assertEqual(snapshot['pragmas']['pageCount'],
                             snapshot['reconciliation']['btreePages']+snapshot['pragmas']['freelistCount'])
            retained = snapshot['retained']
            self.assertEqual(retained['memberReferenceBytes'],8)
            self.assertEqual(retained['distinctReferencedSourceBytes'],4)
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE package_member SET size=3')
            self.assertEqual(api.snapshot(path)['retained']['memberLengthMismatchCount'],2)

    def test_missing_dbstat_and_unmapped_roots_are_explicit_incomplete_errors(self):
        api = accounting(self)
        original = api.reader
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE sample (id INT)')
            for mode in ('missing','unmapped','root'):
                class Proxy:
                    def __init__(self, db):
                        self.db = db
                    def execute(self, query, *args):
                        if 'FROM dbstat' in query:
                            if mode == 'missing':
                                raise sqlite3.DatabaseError('controlled unavailable dbstat')
                            rows = list(self.db.execute(query,*args))
                            if mode == 'unmapped':
                                return [*rows, ('unmapped-tree',0,0,0,0,0)]
                            return [row for row in rows if row[0] != 'sample']
                        return self.db.execute(query,*args)
                @contextmanager
                def controlled(path):
                    with original(path) as db:
                        yield Proxy(db)
                with self.subTest(mode=mode), patch.object(api,'reader',controlled), \
                     self.assertRaisesRegex(ValueError,'dbstat unavailable|unmapped|unattributed'):
                    api.snapshot(path)

    def test_alias_allows_only_one_expected_name_row_and_no_evidence_change(self):
        api = accounting(self)
        self.assertTrue(hasattr(api,'preservation_exact'), 'fixed alias preservation oracle API is missing')
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('''CREATE TABLE package_source_name (package_sha256 TEXT,source_name TEXT,
                        PRIMARY KEY(package_sha256,source_name));
                    INSERT INTO package_source_name VALUES ('p','original');
                    CREATE TABLE future_evidence (sample TEXT);INSERT INTO future_evidence VALUES ('first');''')
            before = api.logical_snapshot(path,root / 'before')
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("INSERT INTO package_source_name VALUES ('p','alias')")
            after = api.logical_snapshot(path,root / 'alias')
            self.assertTrue(api.preservation_exact(path,before,after,False,{('p','original')},('p','alias')))
            self.assertFalse(api.preservation_exact(path,before,after,True))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE future_evidence SET sample='other'")
            altered = api.logical_snapshot(path,root / 'altered')
            self.assertFalse(api.preservation_exact(path,before,altered,False,{('p','original')},('p','alias')))
            self.assertFalse(api.preservation_exact(path,before,after,False,{('p','original')},('p','wrong')))
            self.assertFalse(api.preservation_exact(path,after,after,False,
                             {('p','original'),('p','alias')},('p','alias')),
                             'alias acceptance requires exactly one newly added name row')

    def test_utf16_is_refused_at_reader_before_accounting_without_database_writes(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            for encoding in ('UTF-16le', 'UTF-16be'):
                path = root / (encoding + '.sqlite')
                with closing(sqlite3.connect(path)) as db, db:
                    db.execute("PRAGMA encoding='" + encoding + "'")
                    db.execute('CREATE TABLE sample (value TEXT)')
                    db.executescript('''CREATE TABLE package (sha256 TEXT,size INT,bytes BLOB);
                        CREATE TABLE source_file (sha256 TEXT,source_path TEXT,manufacturer_id TEXT,len INT,bytes BLOB);
                        CREATE TABLE package_member (package_sha256 TEXT,ordinal INT,path TEXT,role TEXT,source_sha256 TEXT,size INT);''')
                    db.execute('INSERT INTO sample VALUES (?)', ('ASCII λ\0漢',))
                    self.assertEqual(db.execute('PRAGMA encoding').fetchone()[0], encoding)
                original = {p.name: p.read_bytes() for p in root.glob('*.sqlite*')}
                def open_reader():
                    with api.reader(path):
                        self.fail('unsupported encoding must not yield a reader')
                operations = (open_reader, lambda: api.snapshot(path), lambda: api.integrity(path),
                              lambda: api.logical_snapshot(path, root / (encoding + '-rows')),
                              lambda: api.verify_sources(path, []))
                for operation in operations:
                    with self.subTest(encoding=encoding, operation=operation), \
                         patch.object(api, 'objects', side_effect=AssertionError('accounting started')), \
                         self.assertRaisesRegex(ValueError, 'unsupported database encoding.*UTF-8'):
                        operation()
                    self.assertEqual(original, {p.name: p.read_bytes() for p in root.glob('*.sqlite*')})

    def test_utf8_reader_still_admits_exact_logical_byte_accounting(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path = Path(tmp) / 'utf8.sqlite'
            text = 'ASCII λ\0漢'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE sample (value TEXT) STRICT')
                db.execute('INSERT INTO sample VALUES (?)', (text,))
            with api.reader(path) as db:
                self.assertEqual(db.execute('PRAGMA encoding').fetchone(), ('UTF-8',))
                self.assertEqual(db.execute('SELECT value FROM sample').fetchone(), (text,))
            table = next(o for o in api.snapshot(path)['objects'] if o['name'] == 'sample')
            self.assertEqual(table['logicalColumns'],
                             [{'name': 'value', 'textBytes': len(text.encode('utf-8')), 'blobBytes': 0}])

    def test_unsupported_layout_refuses_without_writes_or_sidecar_creation(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            for mode in ('wal','auto_vacuum'):
                path = root / (mode+'.sqlite')
                with closing(sqlite3.connect(path)) as db, db:
                    db.execute('PRAGMA journal_mode=WAL' if mode == 'wal' else 'PRAGMA auto_vacuum=FULL')
                    db.execute('CREATE TABLE sample (id INT)')
                db.close()
                before = {p.name:p.read_bytes() for p in root.iterdir()}
                with self.subTest(mode=mode), self.assertRaisesRegex(ValueError,'unsupported'):
                    api.snapshot(path)
                self.assertEqual(before,{p.name:p.read_bytes() for p in root.iterdir()})


class OracleTests(unittest.TestCase):
    def test_hidden_rowid_changes_with_declared_keys_break_full_preservation(self):
        api = accounting(self)
        layouts = (
            ('text_pk', 'id TEXT PRIMARY KEY,sample ANY', ('key', 'λ\0sample'), 'rowid'),
            ('composite_pk', 'id TEXT,sample ANY,PRIMARY KEY(id,sample)', ('key', 7), 'rowid'),
            ('integer_pk', 'id INTEGER PRIMARY KEY,sample ANY', (1, b'\0blob'), 'rowid'),
            ('no_pk', 'id TEXT,sample ANY', ('key', None), 'rowid'),
            ('shadow_rowid', 'RoWiD TEXT PRIMARY KEY,sample ANY', ('key', 1.25), '_rowid_'),
            ('shadow_two', 'ROWID TEXT,_RoWiD_ ANY,PRIMARY KEY(ROWID,_RoWiD_)', ('key', 9), 'oid'),
        )
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, columns, values, alias in layouts:
                with self.subTest(layout=name):
                    path = root / (name + '.sqlite')
                    with closing(sqlite3.connect(path)) as db, db:
                        db.execute('CREATE TABLE evidence (' + columns + ') STRICT')
                        db.execute('INSERT INTO evidence VALUES (?,?)', values)
                        before_sql = db.execute('SELECT ' + alias + ',* FROM evidence').fetchall()
                    before = api.logical_snapshot(path, root / (name + '-before'))
                    same = api.logical_snapshot(path, root / (name + '-same'))
                    self.assertTrue(api.preservation_exact(path, before, same, True))
                    with closing(sqlite3.connect(path)) as db, db:
                        db.execute('UPDATE evidence SET ' + alias + '=99')
                        after_sql = db.execute('SELECT ' + alias + ',* FROM evidence').fetchall()
                    self.assertEqual(before_sql[0][0], 1)
                    self.assertEqual(after_sql[0][0], 99)
                    if name != 'integer_pk':
                        self.assertEqual(before_sql[0][1:], after_sql[0][1:])
                    after = api.logical_snapshot(path, root / (name + '-after'))
                    self.assertFalse(api.streams_equal(before, after), 'hidden rowid is SQL-visible state')
                    self.assertFalse(api.preservation_exact(path, before, after, True))

    def test_all_rowid_aliases_shadowed_refuse_instead_of_partial_equality(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            for pk in ('', ',PRIMARY KEY(rowid)', ',PRIMARY KEY(rowid,oid)'):
                with self.subTest(pk=pk):
                    folder = root / str(len(list(root.iterdir())))
                    folder.mkdir()
                    path = folder / 'observer.sqlite'
                    with closing(sqlite3.connect(path)) as db, db:
                        db.execute('CREATE TABLE evidence (RoWiD TEXT,_rOwId_ TEXT,OID TEXT' + pk + ')')
                        db.execute("INSERT INTO evidence VALUES ('r','u','o')")
                    original = path.read_bytes()
                    with self.assertRaisesRegex(ValueError, 'unshadowed row identity'):
                        api.logical_snapshot(path, folder / 'rows')
                    self.assertEqual(path.read_bytes(), original)

    def test_without_rowid_with_shadow_aliases_has_explicit_complete_typed_rows(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'observer.sqlite'
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('CREATE TABLE evidence (RoWiD TEXT,_rOwId_ ANY,OID ANY,'
                           'PRIMARY KEY(RoWiD,_rOwId_)) STRICT, WITHOUT ROWID')
                db.execute('INSERT INTO evidence VALUES (?,?,?)', ('λ\0', 7, b'\0payload'))
            before = api.logical_snapshot(path, root / 'before')
            self.assertTrue(api.streams_equal(before, api.logical_snapshot(path, root / 'same')))
            # Same row count and same printable value, but different SQL storage class.
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE evidence SET OID=CAST(OID AS TEXT)")
            after = api.logical_snapshot(path, root / 'after')
            self.assertFalse(api.preservation_exact(path, before, after, True))

    def test_external_bytes_members_and_typed_all_table_streams_detect_corruption(self):
        api = accounting(self)
        self.assertTrue(hasattr(api, 'verify_sources'), 'external-byte/member oracle API is missing')
        self.assertTrue(hasattr(api, 'logical_snapshot'), 'typed full-table oracle API is missing')
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'observer.sqlite'
            payload = b'opaque\0source'
            digest = hashlib.sha256(payload).hexdigest()
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, 'w') as archive:
                archive.writestr('M-0001/', b'')
                archive.writestr('M-0001/source.bin', payload)
            package = stream.getvalue()
            package_sha = hashlib.sha256(package).hexdigest()
            source = root / 'source.knxprod'
            source.write_bytes(package)
            entry = {'id': 'source', 'sha256': package_sha, 'kind': 'package',
                     'retainedPackages': [{'innerPath': None, 'sha256': package_sha, 'members': [
                         {'ordinal': 0, 'path': 'M-0001/source.bin', 'role': 'Unrecognized',
                          'sha256': digest, 'sizeBytes': len(payload)}]}]}
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('''CREATE TABLE package (sha256 TEXT PRIMARY KEY,size INT,bytes BLOB);
                    CREATE TABLE source_file (sha256 TEXT PRIMARY KEY,source_path TEXT,
                        manufacturer_id TEXT,len INT,bytes BLOB);
                    CREATE TABLE package_member (package_sha256 TEXT, ordinal INT,path TEXT,
                        role TEXT,source_sha256 TEXT,size INT);
                    CREATE TABLE future_evidence (id INT PRIMARY KEY,sample TEXT,occurrences INT);''')
                db.execute('INSERT INTO package VALUES (?,?,?)', (package_sha,len(package),package))
                db.execute('INSERT INTO source_file VALUES (?,?,?,?,?)',
                           (digest,'M-0001/source.bin','M-0001',len(payload),payload))
                db.execute('INSERT INTO package_member VALUES (?,?,?,?,?,?)',
                           (package_sha,0,'M-0001/source.bin','Unrecognized',digest,len(payload)))
                db.execute('INSERT INTO future_evidence VALUES (1,?,16)', ('first-λ',))
            self.assertEqual(api.verify_sources(path, [(entry, source)]),
                             {'packagesExact': True, 'sourcesExact': True, 'memberIndexExact': True})
            original = api.logical_snapshot(path, root / 'before')
            self.assertTrue(api.streams_equal(original, api.logical_snapshot(path, root / 'same')))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("UPDATE future_evidence SET sample='other-λ',occurrences=17")
            self.assertFalse(api.streams_equal(original, api.logical_snapshot(path, root / 'changed')))
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE source_file SET bytes=?', (b'X' + payload[1:],))
            self.assertFalse(api.verify_sources(path, [(entry, source)])['sourcesExact'])
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('UPDATE source_file SET bytes=?', (payload,))
                db.execute('UPDATE package_member SET ordinal=1')
            self.assertFalse(api.verify_sources(path, [(entry, source)])['memberIndexExact'])
            with closing(sqlite3.connect(path)) as db, db:
                db.execute('DELETE FROM package_member')
            self.assertFalse(api.verify_sources(path, [(entry, source)])['memberIndexExact'])
            with closing(sqlite3.connect(path)) as db, db:
                db.execute("INSERT INTO source_file VALUES ('extra','extra',NULL,1,X'58')")
            self.assertFalse(api.verify_sources(path, [(entry, source)])['sourcesExact'])


class SourceOracleBudgetTests(unittest.TestCase):
    def fixture(self, root, wrapper=False):
        path = root / 'observer.sqlite'
        payloads = [('M-0001/one.bin', b'first\0source'), ('M-0001/two.bin', b'second-source')]
        archive_bytes = io.BytesIO()
        with zipfile.ZipFile(archive_bytes, 'w') as archive:
            for name, payload in payloads:
                archive.writestr(name, payload)
        package = archive_bytes.getvalue()
        sha = hashlib.sha256(package).hexdigest()
        raw = package
        if wrapper:
            outer_bytes = io.BytesIO()
            with zipfile.ZipFile(outer_bytes, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
                archive.writestr('inner.knxprod', package)
            raw = outer_bytes.getvalue()
        source = root / 'input.zip'
        source.write_bytes(raw)
        members = [{'ordinal': i, 'path': name, 'role': 'Unrecognized',
                    'sha256': hashlib.sha256(payload).hexdigest(), 'sizeBytes': len(payload)}
                   for i, (name, payload) in enumerate(payloads)]
        entry = {'sha256': hashlib.sha256(raw).hexdigest(), 'retainedPackages': [
            {'innerPath': 'inner.knxprod' if wrapper else None, 'sha256': sha, 'members': members}]}
        rows = {
            'package': [(sha, len(package), package)],
            'source_file': [(m['sha256'], name, 'M-0001', len(payload), payload)
                            for m, (name, payload) in zip(members, payloads)],
            'package_member': [(sha, m['ordinal'], m['path'], m['role'], m['sha256'], m['sizeBytes'])
                               for m in members],
        }
        with closing(sqlite3.connect(path)) as db, db:
            db.executescript('''CREATE TABLE package (sha256 TEXT PRIMARY KEY,size INT,bytes BLOB);
                CREATE TABLE source_file (sha256 TEXT PRIMARY KEY,source_path TEXT,manufacturer_id TEXT,len INT,bytes BLOB);
                CREATE TABLE package_member (package_sha256 TEXT,ordinal INT,path TEXT,role TEXT,source_sha256 TEXT,size INT);''')
            for table, values in rows.items():
                db.executemany('INSERT INTO ' + table + ' VALUES (' + ','.join('?' for _ in values[0]) + ')', values)
        # Independently count every returned SQL value, not declared blob lengths.
        db_bytes = sum(len(value if isinstance(value, bytes) else str(value).encode('utf-8'))
                       for values in rows.values() for row in values for value in row if value is not None)
        return path, [(entry, source)], {'input': len(raw), 'packages': len(package),
            'decoded': sum(len(p) for _, p in payloads), 'member': max(len(p) for _, p in payloads),
            'database': db_bytes, 'records': sum(map(len, rows.values()))}

    def check_boundary(self, api, path, admitted, constant, limit, error):
        original = path.read_bytes()
        with patch.object(api, constant, limit, create=True):
            self.assertEqual(api.verify_sources(path, admitted),
                             {'packagesExact': True, 'sourcesExact': True, 'memberIndexExact': True})
        with patch.object(api, constant, limit - 1, create=True), self.assertRaisesRegex(ValueError, error):
            api.verify_sources(path, admitted)
        self.assertEqual(path.read_bytes(), original)
        # A refused call must not consume a persistent budget or poison later checks.
        self.assertTrue(all(api.verify_sources(path, admitted).values()))

    def test_source_oracle_input_bytes_are_aggregate_even_for_identical_retries(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            self.check_boundary(api, path, admitted * 2, 'SOURCE_ORACLE_MAX_INPUT_BYTES',
                                sizes['input'] * 2, 'source oracle input byte limit')

    def test_source_oracle_retained_inner_package_bytes_are_bounded(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp), wrapper=True)
            self.check_boundary(api, path, admitted, 'SOURCE_ORACLE_MAX_PACKAGE_BYTES',
                                sizes['packages'], 'source oracle package byte limit')

    def test_source_oracle_decoded_bytes_are_aggregate_not_deduplicated(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            self.check_boundary(api, path, admitted * 2, 'SOURCE_ORACLE_MAX_DECODED_BYTES',
                                sizes['decoded'] * 2, 'source oracle decoded byte limit')

    def test_source_oracle_individual_decoded_member_boundary(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            self.check_boundary(api, path, admitted, 'SOURCE_ORACLE_MAX_MEMBER_BYTES',
                                sizes['member'], 'source oracle member byte limit')

    def test_source_oracle_database_actual_retained_values_have_aggregate_boundary(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            self.check_boundary(api, path, admitted, 'SOURCE_ORACLE_MAX_DB_RETAINED_BYTES',
                                sizes['database'], 'source oracle database retained byte limit')
            # Dishonest stored length fields must not bypass the bytes actually loaded.
            with closing(sqlite3.connect(path)) as db, db:
                db.executescript('UPDATE package SET size=0;UPDATE source_file SET len=0;')
            with patch.object(api, 'SOURCE_ORACLE_MAX_DB_RETAINED_BYTES', sizes['input'] - 1, create=True), \
                 self.assertRaisesRegex(ValueError, 'source oracle database retained byte limit'):
                api.verify_sources(path, admitted)

    def test_source_oracle_database_record_boundary_prevents_empty_row_fanout(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            self.check_boundary(api, path, admitted, 'SOURCE_ORACLE_MAX_RECORDS',
                                sizes['records'], 'source oracle record limit')

    def test_source_oracle_preflight_refuses_before_external_or_database_payload_loading(self):
        api = accounting(self)
        original_reader = api.reader
        original_open = Path.open
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            source = admitted[0][1]
            def checked_open(target, *args, **kwargs):
                self.assertNotEqual(target, source, 'external payload loaded before budget refusal')
                return original_open(target, *args, **kwargs)
            class Proxy:
                def __init__(self, db):
                    self.db = db
                def execute(proxy, sql, *args):
                    self.assertFalse(sql.startswith(('SELECT sha256,', 'SELECT package_sha256,')),
                                     'database payload loaded before budget refusal')
                    return proxy.db.execute(sql, *args)
            @contextmanager
            def controlled(target):
                with original_reader(target) as db:
                    yield Proxy(db)
            cases = (
                ('SOURCE_ORACLE_MAX_INPUT_BYTES', sizes['input'] - 1, 'input byte'),
                ('SOURCE_ORACLE_MAX_DECODED_BYTES', sizes['decoded'] - 1, 'decoded byte'),
                ('SOURCE_ORACLE_MAX_MEMBER_BYTES', sizes['member'] - 1, 'member byte'),
                ('SOURCE_ORACLE_MAX_DB_RETAINED_BYTES', sizes['database'] - 1, 'database retained byte'),
                ('SOURCE_ORACLE_MAX_RECORDS', sizes['records'] - 1, 'record'),
            )
            for constant, limit, message in cases:
                with self.subTest(constant=constant), patch.object(api, constant, limit, create=True), \
                     patch.object(api, 'reader', controlled), patch.object(Path, 'open', checked_open), \
                     self.assertRaisesRegex(ValueError, 'source oracle ' + message + ' limit'):
                    api.verify_sources(path, admitted)

    def test_source_oracle_actual_zip_sizes_cannot_bypass_manifest_budgets(self):
        api = accounting(self)
        with TemporaryDirectory() as tmp:
            path, admitted, sizes = self.fixture(Path(tmp))
            members = admitted[0][0]['retainedPackages'][0]['members']
            # First member authenticates; only the later member understates its size.
            members[1]['sizeBytes'] = 0
            with patch.object(api, 'SOURCE_ORACLE_MAX_DECODED_BYTES', sizes['decoded'] - 1, create=True), \
                 self.assertRaisesRegex(ValueError, 'source oracle decoded byte limit'):
                api.verify_sources(path, admitted)
            with patch.object(api, 'SOURCE_ORACLE_MAX_MEMBER_BYTES', sizes['member'] - 1, create=True), \
                 self.assertRaisesRegex(ValueError, 'source oracle member byte limit'):
                api.verify_sources(path, admitted)


if __name__ == '__main__':
    unittest.main()
