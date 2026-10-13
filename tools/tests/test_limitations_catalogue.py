#!/usr/bin/env python3
"""Regression tests for the generated limitations overview."""

import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))


class CatalogueTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'docs/limitations').mkdir(parents=True)
        (self.root / 'docs/status').mkdir()
        self.entry = {
            'key': 'KL-013', 'number': 13, 'title': 'Protected projects',
            'topic': 'Import and export', 'summary': 'AES stays refused.',
            'file': 'KL-013-protected-projects.md', 'ledger_ids': ['KL-13'],
            'anchors': ['13-password-protected-projects'],
        }
        self.data = {'version': 1, 'entries': [self.entry]}
        self.save()
        (self.root / 'docs/limitations' / self.entry['file']).write_text('# Details\n')
        self.ledger('ACCEPTED_BOUNDARY')

    def save(self):
        (self.root / 'docs/limitations/catalogue.json').write_text(json.dumps(self.data))

    def ledger(self, status):
        (self.root / 'docs/status/LEDGER.md').write_text(
            '# Ledger\n\n## Snapshot IDs\n\n'
            '| ID | P | Owner | Route | Status | Owner disposition | Evidence and remaining work |\n'
            f'| `KL-13` | P1 | alpha | AR08 | {status} | — | evidence |\n'
        )

    def module(self):
        spec = importlib.util.find_spec('limitations_catalogue')
        self.assertIsNotNone(spec, 'Missing limitations overview generator')
        import limitations_catalogue
        return limitations_catalogue

    def test_overview_derives_status_and_links_to_individual_detail(self):
        rendered = self.module().render(self.root)
        self.assertIn('### 13. Protected projects', rendered)
        self.assertIn('AES stays refused.', rendered)
        self.assertIn('ACCEPTED_BOUNDARY', rendered)
        self.assertIn('(limitations/KL-013-protected-projects.md)', rendered)
        self.assertIn('<a id="13-password-protected-projects"></a>', rendered)
        self.ledger('DONE')
        changed = self.module().render(self.root)
        self.assertIn('DONE', changed)
        self.assertNotIn('ACCEPTED_BOUNDARY', changed)

    def test_check_detects_a_stale_ledger_projection(self):
        module = self.module()
        self.assertTrue(hasattr(module, 'check'), 'Missing catalogue drift check')
        (self.root / 'docs/KNOWN_LIMITATIONS.md').write_text(module.render(self.root))
        self.assertEqual(module.check(self.root), [])
        self.ledger('DONE')
        self.assertTrue(any('stale' in error.lower() for error in module.check(self.root)))

    def test_invalid_catalogue_inputs_are_refused(self):
        module = self.module()
        mutations = {
            'unsupported version': lambda: self.data.update(version=2),
            'empty catalogue': lambda: self.data.update(entries=[]),
            'duplicate key': lambda: self.data['entries'].append(dict(self.entry)),
            'duplicate file': lambda: self.data['entries'].append(dict(self.entry, key='other')),
            'path traversal': lambda: self.entry.update(file='../outside.md'),
            'unknown topic': lambda: self.entry.update(topic='Imaginary category'),
            'stored status': lambda: self.entry.update(status='DONE'),
            'missing detail': lambda: self.entry.update(file='missing.md'),
            'empty summary': lambda: self.entry.update(summary=''),
            'invalid anchor': lambda: self.entry.update(anchors=['bad\" onclick=\"oops']),
        }
        original = json.dumps(self.data)
        for name, mutation in mutations.items():
            with self.subTest(name=name):
                self.data = json.loads(original)
                self.entry = self.data['entries'][0]
                mutation()
                self.save()
                with self.assertRaises(ValueError, msg=name):
                    module.render(self.root)

    def test_untracked_notice_never_invents_a_disposition(self):
        self.entry['ledger_ids'] = []
        self.save()
        rendered = self.module().render(self.root)
        self.assertIn('Not separately tracked.', rendered)
        self.assertNotIn('**Ledger status:** `DONE`', rendered)

    def test_reconciliation_history_cannot_override_current_status(self):
        path = self.root / 'docs/status/LEDGER.md'
        with path.open('a') as file:
            file.write('\n## Reconciliation record\n| `KL-13` | P1 | alpha | AR08 | DONE | — | old |\n')
        self.assertIn('ACCEPTED_BOUNDARY', self.module().render(self.root))

    def test_a_missing_current_ledger_row_is_a_reported_error(self):
        self.entry['ledger_ids'] = ['KL-999']
        self.save()
        errors = self.module().check(self.root)
        self.assertTrue(any('KL-999' in error for error in errors))

    def test_documentation_gate_rejects_catalogue_drift(self):
        import check_documentation
        manual = self.root / 'docs/manual'
        manual.mkdir()
        (manual / 'README.md').write_text('1. [Known issues](known-issues.md)\n')
        (manual / 'known-issues.md').write_text('Previous: [Index](README.md)\n# Issues\nNext: [Index](README.md)\n')
        (self.root / 'docs/OPEN_WORK.md').write_text('# Work\n')
        (self.root / 'docs/KNOWN_LIMITATIONS.md').write_text(self.module().render(self.root))
        self.ledger('DONE')
        _, errors = check_documentation.check(self.root)
        self.assertTrue(any('stale limitations' in error.lower() for error in errors), errors)

    def test_duplicate_or_invalid_ledger_rows_are_refused(self):
        path = self.root / 'docs/status/LEDGER.md'
        with path.open('a') as file:
            file.write('| `KL-13` | P1 | alpha | AR08 | DONE | — | duplicate |\n')
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            self.module().render(self.root)
        self.ledger('MADE_UP')
        with self.assertRaisesRegex(ValueError, 'status'):
            self.module().render(self.root)

    def test_orphan_detail_file_is_refused(self):
        (self.root / 'docs/limitations/orphan.md').write_text('# Orphan\n')
        with self.assertRaisesRegex(ValueError, 'Unlisted'):
            self.module().render(self.root)

    def test_closed_aliases_belong_to_the_historical_dossier(self):
        project = TOOLS.parent
        entries = self.module().catalogue_entries(project)
        first = next(e for e in entries if e['key'] == 'KL-001')
        closed = next(e for e in entries if e['key'] == 'closed-entries-and-legacy-links')
        alias = '10-the-project-licence-is-not-decided'
        self.assertNotIn(alias, first['anchors'])
        self.assertIn(alias, closed['anchors'])

    def test_every_migrated_record_matches_its_preservation_receipt(self):
        project = TOOLS.parent
        receipt = json.loads((project / 'docs/evidence/limitations-reorganization-2026-10-13.json').read_text())
        entries = self.module().catalogue_entries(project)
        self.assertTrue({r['key'] for r in receipt['sections']} <= {e['key'] for e in entries})
        overview = (project / 'docs/KNOWN_LIMITATIONS.md').read_text()
        for record in receipt['sections']:
            with self.subTest(key=record['key']):
                body = (project / record['detail_file']).read_text()
                self.assertEqual(body.count('<!-- BEGIN retained catalogue record -->'), 1)
                retained = body.split('<!-- BEGIN retained catalogue record -->\n', 1)[1].split('\n<!-- END retained catalogue record -->', 1)[0]
                self.assertEqual(hashlib.sha256(retained.encode()).hexdigest(), record['retained_sha256'])
                for anchor in record['anchors']:
                    self.assertIn(f'<a id="{anchor}"></a>', overview)
        for record in receipt['resolved_history']['sections']:
            with self.subTest(history=record['heading']):
                body = (project / record['detail_file']).read_text()
                self.assertIn('<!-- BEGIN earlier resolved record -->', body)
                retained = body.split('<!-- BEGIN earlier resolved record -->\n', 1)[1].split('\n<!-- END earlier resolved record -->', 1)[0]
                self.assertEqual(hashlib.sha256(retained.encode()).hexdigest(), record['retained_sha256'])

    def test_ci_runs_status_and_history_checks(self):
        workflow = (TOOLS.parent / '.github/workflows/ci.yml').read_text()
        self.assertIn('python3 tools/check_documentation.py', workflow)
        self.assertIn('python3 -m unittest discover -s tools/tests -p test_limitations_catalogue.py', workflow)

    def test_legacy_alias_cannot_shadow_a_dossier_anchor(self):
        self.entry['anchors'].append('catalogue-kl-013')
        self.save()
        with self.assertRaisesRegex(ValueError, 'anchor'):
            self.module().render(self.root)

    def test_case_distinct_keys_cannot_share_a_dossier_anchor(self):
        alternative = dict(self.entry, key='kl-013', file='alternative.md', anchors=[])
        (self.root / 'docs/limitations/alternative.md').write_text('# Alternative\n')
        self.data['entries'].append(alternative)
        self.save()
        with self.assertRaisesRegex(ValueError, 'key'):
            self.module().render(self.root)


if __name__ == '__main__':
    unittest.main()
