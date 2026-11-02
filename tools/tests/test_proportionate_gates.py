"""Source-bound proportionate gates keep narrow data changes separate from executable risk."""
from __future__ import annotations

import json
import copy
from pathlib import Path
import subprocess
import unittest

from tools import gate_scope
from tools.tests.test_gate_scope import Fixture, MEMBERS

WEB = 'apps/knx-web/'


class ProportionateGates(unittest.TestCase):
    def setUp(self):
        self.fixture = Fixture()
        self.addCleanup(self.fixture.close)
        self.root = self.fixture.root
        self.members = {**MEMBERS, 'knx-desktop': {'dir': 'apps/knx-desktop', 'deps': {'knx-server'}}}
        self.manifest = {'name': 'tiny-web', 'version': '1.0.0', 'scripts': {'build': 'vite'},
                         'dependencies': {'react': '1.0.0'}}
        self.lock = {'name': 'tiny-web', 'version': '1.0.0', 'lockfileVersion': 3, 'requires': True,
                     'packages': {'': self.manifest.copy(), 'node_modules/react': {'version': '1.0.0'}}}
        self.write_json()
        self.git('init', '-q')
        self.git('add', '.')
        self.git('commit', '-qm', 'base')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')

    def git(self, *args):
        return subprocess.run(['git', '-c', 'user.name=t', '-c', 'user.email=t@example.invalid', *args],
                              cwd=self.root, capture_output=True, text=True, check=True).stdout.strip()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def write_json(self):
        self.write(WEB + 'package.json', json.dumps(self.manifest))
        self.write(WEB + 'package-lock.json', json.dumps(self.lock))

    def font(self):
        self.manifest['dependencies']['@fontsource/inter'] = '^5.0.0'
        self.lock['packages'][''] = self.manifest.copy()
        self.lock['packages']['node_modules/@fontsource/inter'] = {
            'version': '5.0.0', 'resolved': 'https://registry.npmjs.org/@fontsource/inter/-/inter-5.0.0.tgz',
            'integrity': 'sha512-fixture', 'license': 'OFL-1.1'}
        self.write_json()

    def plan(self, base='origin/main'):
        return gate_scope.plan(self.root, gate_scope.changed_paths(self.root, base), self.members,
                               comparison_base=base)

    def test_font_only_delta_keeps_default_lane_and_one_production_smoke(self):
        self.font()
        result = self.plan()
        self.assertEqual(result.production_configs, ['dialogs'])
        self.assertTrue(result.web)
        self.assertIsNone(result.browser_specs)
        self.assertEqual(result.rust_packages, [])
        self.assertFalse(result.full)

    def test_html_keeps_default_lane_and_unions_changed_production_helper(self):
        self.write(WEB + 'index.html', '<title>New shell</title>')
        self.assertEqual(self.plan().production_configs, ['dialogs'])
        self.write(WEB + 'e2e/helper.ts', 'export const open = (page) => page.goto("/"); // edit')
        result = self.plan()
        self.assertIsNone(result.browser_specs)
        self.assertEqual(result.production_configs, ['dialogs', 'devices'])

    def readers(self):
        source = Path(__file__).resolve().parents[2]
        for name in ('appimage.rs', 'ledger.rs'):
            self.write('xtask/src/' + name, (source / 'xtask/src' / name).read_text())
        self.write('docs/KNOWN_LIMITATIONS.md', '# Before')
        self.git('add', '.')
        self.git('commit', '-qm', 'reader base')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')

    def test_audited_version_projection_and_ledger_data_do_not_select_xtask(self):
        self.readers()
        self.font()
        self.write('docs/KNOWN_LIMITATIONS.md', '# After')
        self.assertEqual(self.plan().rust_packages, [])

    def diagnostic(self):
        source = Path(__file__).resolve().parents[2]
        for name in ('diagnostic_bundle_sections.rs', 'diagnostic_bundle_sections_tests.rs'):
            self.write('apps/knx-server/src/' + name, (source / 'apps/knx-server/src' / name).read_text())
        self.git('add', '.')
        self.git('commit', '-qm', 'diagnostic base')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        path = self.root / 'apps/knx-server/src/diagnostic_bundle_sections.rs'
        path.write_text(path.read_text().replace('    "porcelain",', '    "rail",\n    "porcelain",'))

    def test_registry_data_selects_no_whole_crate_or_corpus(self):
        self.diagnostic()
        result = self.plan()
        self.assertEqual(result.rust_packages, [])
        self.assertEqual(result.corpus_packages, [])
        self.assertIn('diagnostic-sections-tests', str(result.reasons))

    def broad_font(self):
        self.assertEqual(self.plan().production_configs, list(gate_scope.PRODUCTION_CONFIGS))

    def test_committed_staged_and_unstaged_font_changes_use_merge_base(self):
        self.font()
        self.git('add', '.')
        self.git('commit', '-qm', 'font')
        self.assertEqual(self.plan().production_configs, ['dialogs'])
        self.git('update-ref', 'refs/heads/other-base', 'HEAD')
        self.manifest['dependencies']['@fontsource/inter'] = '^5.1.0'
        self.lock['packages']['node_modules/@fontsource/inter']['version'] = '5.1.0'
        self.write_json()
        self.git('add', WEB + 'package.json')
        self.assertEqual(self.plan('other-base').production_configs, ['dialogs'])

    def test_font_variable_namespace_is_admitted(self):
        self.font()
        self.manifest['dependencies']['@fontsource-variable/inter'] = self.manifest['dependencies'].pop('@fontsource/inter')
        self.lock['packages']['node_modules/@fontsource-variable/inter'] = self.lock['packages'].pop('node_modules/@fontsource/inter')
        self.lock['packages']['node_modules/@fontsource-variable/inter']['resolved'] = \
            'https://registry.npmjs.org/@fontsource-variable/inter/-/inter-5.0.0.tgz'
        self.write_json()
        self.assertEqual(self.plan().production_configs, ['dialogs'])

    def test_font_removal_is_admitted(self):
        self.font()
        self.git('add', '.')
        self.git('commit', '-qm', 'font baseline')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.manifest['dependencies'].pop('@fontsource/inter')
        self.lock['packages'].pop('node_modules/@fontsource/inter')
        self.write_json()
        self.assertEqual(self.plan().production_configs, ['dialogs'])

    def test_lock_only_font_update_is_admitted_against_unchanged_root_maps(self):
        self.font()
        self.git('add', '.')
        self.git('commit', '-qm', 'font baseline')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        record = self.lock['packages']['node_modules/@fontsource/inter']
        record['version'] = '5.1.0'
        record['resolved'] = record['resolved'].replace('5.0.0', '5.1.0')
        self.write_json()
        self.assertEqual(self.plan().production_configs, ['dialogs'])

    def test_font_mixed_manifest_changes_are_broad(self):
        self.font()
        original = copy.deepcopy(self.manifest)
        for key, value in [('scripts', {'build': 'unsafe'}), ('engines', {'node': '>=99'}),
                           ('version', '2.0.0'), ('dependencies', {'react': '2.0.0', '@fontsource/inter': '^5.0.0'})]:
            with self.subTest(key=key):
                self.manifest = {**copy.deepcopy(original), key: value}
                self.write_json()
                self.broad_font()

    def test_font_nonregistry_specs_are_broad(self):
        self.font()
        for value in ('file:../font', 'npm:executable@1.0.0', 'https://example.invalid/x.tgz', '*'):
            with self.subTest(value=value):
                self.manifest['dependencies']['@fontsource/inter'] = value
                self.write_json()
                self.broad_font()

    def test_font_lock_metadata_executable_and_transitive_changes_are_broad(self):
        self.font()
        original = copy.deepcopy(self.lock)
        variants = []
        for key, value in [('hasInstallScript', True), ('scripts', {'install': 'bad'}),
                           ('dependencies', {'exe': '1'}), ('optionalDependencies', {'exe': '1'}),
                           ('peerDependencies', {'exe': '1'}), ('bin', 'execute.js'), ('unknown', 1)]:
            lock = copy.deepcopy(original)
            lock['packages']['node_modules/@fontsource/inter'][key] = value
            variants.append(lock)
        for key, value in [('lockfileVersion', 2), ('requires', 1), ('extra', True)]:
            variants.append({**copy.deepcopy(original), key: value})
        lock = copy.deepcopy(original)
        lock['packages']['node_modules/exe'] = {'version': '1.0.0'}
        variants.append(lock)
        lock = copy.deepcopy(original)
        lock['packages']['node_modules/exe'] = None
        variants.append(lock)
        lock = copy.deepcopy(original)
        lock['packages']['node_modules/@fontsource/inter/node_modules/exe'] = {'version': '1.0.0'}
        variants.append(lock)
        for index, lock in enumerate(variants):
            with self.subTest(index=index):
                self.lock = lock
                self.write_json()
                self.broad_font()

    def test_font_root_maps_must_match_both_manifests(self):
        # Deliberately mismatch the committed old root as well as the final root.
        self.lock['packages'][''] = {'dependencies': {'react': 'different'}}
        self.write_json()
        self.git('add', '.')
        self.git('commit', '-qm', 'mismatched baseline')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.font()
        self.broad_font()
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD~1')
        self.lock['packages'][''] = {'dependencies': {'react': 'different'}}
        self.write_json()
        self.broad_font()

    def test_malformed_duplicate_missing_and_deleted_json_are_broad(self):
        self.font()
        good = (self.root / (WEB + 'package.json')).read_text()
        for text in ('{', '[]', good[:-1] + ', "version": "1.0.0"}', '{"version":NaN}'):
            with self.subTest(text=text):
                self.write(WEB + 'package.json', text)
                self.broad_font()
        self.write_json()
        self.git('mv', WEB + 'package.json', WEB + 'renamed.json')
        self.assertIn(WEB + 'package.json', gate_scope.changed_paths(self.root, 'origin/main'))
        self.assertTrue(self.plan().full)
        self.assertEqual(self.plan().production_configs, list(gate_scope.PRODUCTION_CONFIGS))

    def test_missing_baseline_and_new_untracked_manifest_fall_back(self):
        self.font()
        result = gate_scope.plan(self.root, [WEB + 'package.json', WEB + 'package-lock.json'],
                                 self.members, comparison_base='absent-ref')
        self.assertEqual(result.production_configs, list(gate_scope.PRODUCTION_CONFIGS))
        self.git('rm', '--cached', WEB + 'package.json')
        self.git('commit', '-qm', 'no manifest baseline')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.broad_font()

    def test_deleted_or_unavailable_html_and_real_build_inputs_stay_broad(self):
        (self.root / (WEB + 'index.html')).unlink()
        self.assertEqual(self.plan().production_configs, list(gate_scope.PRODUCTION_CONFIGS))
        self.write(WEB + 'index.html', '<title>shell</title>')
        self.write(WEB + 'vite.config.ts', 'export default {};')
        self.assertEqual(set(self.plan().production_configs), set(gate_scope.PRODUCTION_CONFIGS))

    def test_reader_changed_version_changed_hash_and_additional_reader_remain_selected(self):
        self.readers()
        self.font()
        self.manifest['version'] = '2.0.0'
        self.write_json()
        self.assertIn('xtask', self.plan().rust_packages)
        self.manifest['version'] = '1.0.0'
        self.write_json()
        path = self.root / 'xtask/src/appimage.rs'
        original = path.read_text()
        path.write_text(original + '\n// reader changed\n')
        self.assertIn('xtask', self.plan().rust_packages)
        path.write_text(original)
        self.write('xtask/src/extra.rs', 'const INPUT: &str = "apps/knx-web/package.json";')
        # Even if the extra reader was committed into the baseline, its read remains.
        self.git('add', 'xtask/src/extra.rs')
        self.git('commit', '-qm', 'additional reader')
        self.assertIn('xtask', self.plan().rust_packages)

    def test_ledger_changed_hash_missing_input_and_additional_reader_remain_selected(self):
        self.readers()
        path = self.root / 'xtask/src/ledger.rs'
        original = path.read_text()
        path.write_text(original + '\n// changed\n')
        self.write('docs/KNOWN_LIMITATIONS.md', '# After')
        self.assertIn('xtask', self.plan().rust_packages)
        path.write_text(original)
        (self.root / 'docs/KNOWN_LIMITATIONS.md').unlink()
        self.assertIn('xtask', self.plan().rust_packages)
        self.write('docs/KNOWN_LIMITATIONS.md', '# After')
        self.write('crates/knx-core/tests/extra.rs', 'const X: &str = "docs/KNOWN_LIMITATIONS.md";')
        self.assertIn('knx-core', self.plan().rust_packages)
        self.assertNotIn('xtask', self.plan().rust_packages)

    def test_registry_with_new_literal_cases_is_admitted_committed_or_uncommitted(self):
        self.diagnostic()
        path = self.root / 'apps/knx-server/src/diagnostic_bundle_sections_tests.rs'
        path.write_text(path.read_text().replace('        ("tlh",', '        ("bar", "rail", "rail"),\n        ("tlh",'))
        self.assertTrue(self.plan().diagnostic_sections)
        self.git('add', '.')
        self.git('commit', '-qm', 'metadata')
        self.assertTrue(self.plan().diagnostic_sections)

    def broad_diagnostic(self):
        result = self.plan()
        self.assertEqual(result.rust_packages, ['knx-desktop', 'knx-server'])
        self.assertFalse(result.diagnostic_sections)
        self.assertEqual(result.corpus_packages, ['knx-desktop', 'knx-server'])

    def test_registry_outside_region_or_nonliteral_duplicate_and_assertion_changes_are_broad(self):
        self.diagnostic()
        path = self.root / 'apps/knx-server/src/diagnostic_bundle_sections.rs'
        original = path.read_text()
        for text in (original + '\nfn side_effect() {}\n',
                     original.replace('"rail",', '"porcelain",'),
                     original.replace('"rail",', 'concat!("r", "ail"),'),
                     original.replace('const SHIPPED_THEMES', 'const OTHER_THEMES'),
                     original.replace('const UI_LANGUAGES', 'const OTHER_LANGUAGES')):
            with self.subTest(text=text[-30:]):
                path.write_text(text)
                self.broad_diagnostic()
        path.write_text(original)
        test = self.root / 'apps/knx-server/src/diagnostic_bundle_sections_tests.rs'
        test.write_text(test.read_text().replace('assert_eq!(body["theme"], expected_theme);',
                                                'assert_eq!(body["theme"], "other");'))
        self.broad_diagnostic()

    def test_registry_test_only_removal_duplicate_nonliteral_and_outside_changes_are_broad(self):
        self.diagnostic()
        source = self.root / 'apps/knx-server/src/diagnostic_bundle_sections.rs'
        changed = source.read_text()
        tests = self.root / 'apps/knx-server/src/diagnostic_bundle_sections_tests.rs'
        original = tests.read_text()
        for data in (original.replace('        ("tlh", "user-imported-pack", "other"),\n', ''),
                     original.replace('        ("tlh",', '        ("tlh", "user-imported-pack", "other"),\n        ("tlh",'),
                     original.replace('        ("tlh",', '        ("bar", function(), "rail"),\n        ("tlh",'),
                     original + '\nfn unrelated() {}\n'):
            with self.subTest(data=data[-30:]):
                tests.write_text(data)
                self.broad_diagnostic()
        tests.write_text(original.replace('        ("tlh",', '        ("bar", "rail", "rail"),\n        ("tlh",'))
        source.write_text(changed.replace('    "rail",\n', ''))
        self.broad_diagnostic()

    def test_registry_normal_rust_change_and_core_reverse_dependency_win(self):
        self.diagnostic()
        self.write('apps/knx-server/src/extra.rs', '// normal code')
        self.broad_diagnostic()
        (self.root / 'apps/knx-server/src/extra.rs').unlink()
        self.write('crates/knx-core/src/lib.rs', '// changed core')
        result = self.plan()
        self.assertIn('knx-server', result.rust_packages)
        self.assertIn('knx-desktop', result.rust_packages)
        self.assertFalse(result.diagnostic_sections)

    def test_registry_deleted_missing_baseline_and_renamed_tests_are_broad(self):
        self.diagnostic()
        result = gate_scope.plan(self.root, ['apps/knx-server/src/diagnostic_bundle_sections.rs'],
                                 self.members, comparison_base='absent')
        self.assertEqual(result.rust_packages, ['knx-desktop', 'knx-server'])
        self.git('mv', 'apps/knx-server/src/diagnostic_bundle_sections_tests.rs',
                 'apps/knx-server/src/moved_tests.rs')
        self.broad_diagnostic()

    def test_helper_is_gate_infrastructure_and_unknown_changes_stay_full(self):
        self.font()
        for path in ('tools/gate_data_facts.py', 'unknown/input'):
            self.assertTrue(gate_scope.plan(self.root, [path, WEB + 'package.json'], self.members).full)


if __name__ == '__main__':
    unittest.main()
