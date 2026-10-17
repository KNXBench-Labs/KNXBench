"""Tests for lossless documentation partitioning and historical preservation."""

import hashlib
import json
import tempfile
import importlib.util
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


class DocumentationHistoryTests(unittest.TestCase):
    def module(self):
        self.assertIsNotNone(importlib.util.find_spec('documentation_history'),
                             'Missing lossless documentation-history helper')
        import documentation_history
        return documentation_history

    def test_partition_ignores_fenced_heading_examples_and_preserves_all_bytes(self):
        source = '# Log\n\n## 2026-10-13 — latest\n\n```md\n## fake\n```\n\n## 2026-09-30 — late older entry\nbody\n'
        parts = self.module().partition(source, lambda level, title: level == 2)
        self.assertEqual(''.join(part['body'] for part in parts), source)
        self.assertEqual([part['heading'] for part in parts],
                         [None, '2026-10-13 — latest', '2026-09-30 — late older entry'])

    def test_local_links_are_rebased_and_exactly_reversible(self):
        module = self.module()
        self.assertTrue(hasattr(module, 'relocate'), 'Missing relative-link relocation')
        source = '[same](#19-notes) [doc](../DATA_MODEL.md#entities) [site](https://example.org/a)\n'
        moved, edits = module.relocate(source, 'docs/research/commissioning.md',
                                       'docs/research/commissioning/19-notes.md')
        self.assertIn('(../../DATA_MODEL.md#entities)', moved)
        self.assertIn('(../commissioning.md#19-notes)', moved)
        self.assertIn('(https://example.org/a)', moved)
        self.assertEqual(module.restore_links(moved, edits), source)

    def test_heading_aliases_keep_duplicate_suffixes_and_ignore_code(self):
        module = self.module()
        self.assertTrue(hasattr(module, 'anchors'), 'Missing original anchor inventory')
        source = '# Root\n## Example\n## Example\n```md\n## Not a heading\n```\n<a id="old-name"></a>\n'
        self.assertEqual([a['id'] for a in module.anchors(source)],
                         ['root', 'example', 'example-1', 'old-name'])

    def test_receipt_checks_exact_preservation_and_detects_a_removed_sentence(self):
        module = self.module()
        self.assertTrue(hasattr(module, 'check'), 'Missing history preservation gate')
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'docs/evidence').mkdir(parents=True)
            original = '## Entry\nThe original evidence stays.\n'
            digest = hashlib.sha256(original.encode()).hexdigest()
            record = {'key': 'example', 'file': 'docs/archive.md', 'source_offset': 0,
                      'source_length': len(original), 'source_sha256': digest,
                      'retained_sha256': digest, 'link_edits': [], 'anchors': ['entry']}
            receipt = {'version': 1, 'sources': [{'source': 'docs/index.md',
                       'sha256': digest, 'records': [record]}]}
            path = root / 'docs/archive.md'
            path.write_text('<!-- BEGIN preserved example -->\n' + original + '\n<!-- END preserved example -->\n')
            (root / 'docs/index.md').write_text('<a id="entry"></a>\n[Details](archive.md)\n')
            (root / 'docs/evidence/documentation-reorganization-2026-10-13.json').write_text(json.dumps(receipt))
            self.assertEqual(module.check(root), [])
            path.write_text(path.read_text().replace('The original evidence stays.\n', ''))
            self.assertTrue(any('example' in error for error in module.check(root)))

    def test_a_declared_redaction_replaces_byte_identity_for_that_record_only(self):
        module = self.module()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'docs/evidence').mkdir(parents=True)
            first, second = '## One\nBuilt in /secret/place.\n', '## Two\nUntouched.\n'
            sha = lambda text: hashlib.sha256(text.encode()).hexdigest()
            redacted = first.replace('/secret/place', '<sources>')
            records = [
                {'key': 'one', 'file': 'docs/archive.md', 'source_offset': 0,
                 'source_length': len(first), 'source_sha256': sha(first),
                 'retained_sha256': sha(redacted), 'link_edits': [], 'anchors': [],
                 'redaction': {'reason': 'local machine details removed',
                               'previous_retained_sha256': sha(first)}},
                {'key': 'two', 'file': 'docs/archive.md', 'source_offset': len(first),
                 'source_length': len(second), 'source_sha256': sha(second),
                 'retained_sha256': sha(second), 'link_edits': [], 'anchors': []}]
            receipt = {'version': 1, 'sources': [{'source': 'docs/index.md',
                       'sha256': sha(first + second), 'records': records}]}
            receipt_path = root / 'docs/evidence/documentation-reorganization-2026-10-13.json'
            path = root / 'docs/archive.md'
            body = lambda one, two: (f'<!-- BEGIN preserved one -->\n{one}\n<!-- END preserved one -->\n'
                                     f'<!-- BEGIN preserved two -->\n{two}\n<!-- END preserved two -->\n')
            (root / 'docs/index.md').write_text('[Details](archive.md)\n')
            path.write_text(body(redacted, second))
            receipt_path.write_text(json.dumps(receipt))
            self.assertEqual(module.check(root), [])
            # The unredacted neighbour keeps its full byte-identity check.
            path.write_text(body(redacted, '## Two\nEdited.\n'))
            self.assertTrue(any('two' in error for error in module.check(root)))
            # The redacted record is pinned to its new bytes, too.
            path.write_text(body(redacted.replace('Built', 'Made'), second))
            self.assertTrue(any('one' in error for error in module.check(root)))
            # A redaction must say why and name the bytes it replaced.
            path.write_text(body(redacted, second))
            records[0]['redaction'] = {'reason': ' ', 'previous_retained_sha256': sha(first)}
            receipt_path.write_text(json.dumps(receipt))
            self.assertTrue(any('redaction' in error for error in module.check(root)))
            records[0]['redaction'] = {'reason': 'x', 'previous_retained_sha256': 'nope'}
            receipt_path.write_text(json.dumps(receipt))
            self.assertTrue(any('redaction' in error for error in module.check(root)))

    def test_real_archive_preservation_and_old_fragment_targets(self):
        module = self.module()
        self.assertTrue(hasattr(module, 'check'), 'Missing history preservation gate')
        self.assertEqual(module.check(Path(__file__).resolve().parents[2]), [])

    def test_documentation_gate_runs_history_verification(self):
        import check_documentation
        from unittest.mock import patch as mock_patch
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            manual = root / 'docs/manual'
            manual.mkdir(parents=True)
            (manual / 'README.md').write_text('1. [One](one.md)\n')
            (manual / 'one.md').write_text('Previous: [Index](README.md)\n# One\nNext: [Index](README.md)\n')
            (root / 'docs/evidence').mkdir()
            (root / 'docs/evidence/documentation-reorganization-2026-10-13.json').write_text('{}')
            with mock_patch.object(self.module(), 'check', return_value=['Named historical loss']) as called:
                _, errors = check_documentation.check(root)
                self.assertIn('Named historical loss', errors)
                called.assert_called_once_with(root.resolve())

    def test_ci_runs_history_regressions(self):
        workflow = (Path(__file__).resolve().parents[2] / '.github/workflows/ci.yml').read_text()
        self.assertIn('python3 -m unittest discover -s tools/tests -p test_documentation_history.py', workflow)

    def test_empty_original_guide_does_not_create_a_dead_index_section(self):
        index = (Path(__file__).resolve().parents[2] / 'docs/IMPLEMENTATION_STATUS.md').read_text()
        self.assertNotIn('## Original reading guide\n\n##', index)

    def test_original_aliases_point_to_their_actual_dossier(self):
        root = Path(__file__).resolve().parents[2]
        receipt = json.loads((root / 'docs/evidence/documentation-reorganization-2026-10-13.json').read_text())
        for source in receipt['sources']:
            if source['source'] == '.ai/CURRENT_STATE.md':
                continue
            index = (root / source['source']).read_text()
            from urllib.parse import urlsplit
            import posixpath
            for record in source['records']:
                for alias in record['anchors']:
                    remaining = index.split(f'<a id="{alias}"></a>', 1)[1]
                    line = next(line for line in remaining.splitlines() if '](' in line)
                    target = line.split('](', 1)[1].split(')', 1)[0]
                    self.assertEqual(posixpath.normpath(posixpath.join(
                        posixpath.dirname(source['source']), urlsplit(target).path)), record['file'])


if __name__ == '__main__':
    unittest.main()
