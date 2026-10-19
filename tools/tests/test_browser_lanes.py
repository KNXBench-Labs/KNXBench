"""Browser lane guard: real-time measurements must carry the @exclusive tag."""
from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from tools import browser_lanes as lanes

ROOT = Path(__file__).resolve().parents[2]


def scan(source: str, helpers: dict[str, str] | None = None) -> list[str]:
    """Return 'line:primitive' strings for every untagged real-time use."""
    with tempfile.TemporaryDirectory() as directory:
        e2e = Path(directory)
        (e2e / 'spec.e2e.ts').write_text(source)
        for name, text in (helpers or {}).items():
            (e2e / name).write_text(text)
        return [f'{f.line}:{f.primitive}' for f in lanes.check_directory(e2e)]


class BrowserLaneGuardTests(unittest.TestCase):
    def test_untagged_real_time_wait_is_refused(self) -> None:
        self.assertEqual(scan('test("a", async ({ page }) => {\n  await page.waitForTimeout(250);\n});\n'),
                         ['2:waitForTimeout'])

    def test_every_guarded_primitive_is_recognised(self) -> None:
        source = ('test("a", async ({ page }) => {\n'
                  '  const t = performance.now();\n'
                  '  const d = Date.now();\n'
                  '  await page.evaluate(() => new Promise(requestAnimationFrame));\n'
                  '  await new Promise((r) => setTimeout(r, 5));\n'
                  '  setInterval(() => {}, 5);\n'
                  '});\n')
        self.assertEqual(scan(source), ['2:performance.now', '3:Date.now', '4:requestAnimationFrame',
                                        '5:setTimeout', '6:setInterval'])

    def test_tagged_test_and_tagged_describe_are_admitted(self) -> None:
        self.assertEqual(scan('test("a", { tag: "@exclusive" }, async ({ page }) => {\n'
                              '  await page.waitForTimeout(250);\n});\n'), [])
        self.assertEqual(scan('test("a", { tag: ["@slow", "@exclusive"] }, async ({ page }) => {\n'
                              '  await page.waitForTimeout(250);\n});\n'), [])
        self.assertEqual(scan('test.describe("g", { tag: "@exclusive" }, () => {\n'
                              '  test("a", async ({ page }) => { await page.waitForTimeout(1); });\n'
                              '});\n'), [])

    def test_another_tag_or_an_annotation_is_not_exclusive(self) -> None:
        self.assertEqual(scan('test("a", { tag: "@slow", annotation: { type: "exclusive" } }, async ({ page }) => {\n'
                              '  await page.waitForTimeout(250);\n});\n'), ['2:waitForTimeout'])

    def test_comments_strings_and_regexes_do_not_count(self) -> None:
        source = ('// page.waitForTimeout(1) in a comment\n'
                  '/* Date.now() */\n'
                  'test("Date.now() in a title", async ({ page }) => {\n'
                  '  const text = "performance.now()" + \'setTimeout(\';\n'
                  '  expect(text).toMatch(/setTimeout\\(/);\n'
                  '});\n')
        self.assertEqual(scan(source), [])

    def test_code_inside_a_template_literal_counts(self) -> None:
        self.assertEqual(scan('test("a", async () => {\n  const s = `id-${Date.now()}`;\n});\n'),
                         ['2:Date.now'])

    def test_module_helper_taints_its_untagged_callers_only(self) -> None:
        source = ('async function settle(page) {\n'
                  '  await page.waitForTimeout(100);\n'
                  '}\n'
                  'const twice = async (page) => { await settle(page); await settle(page); };\n'
                  'test("tagged", { tag: "@exclusive" }, async ({ page }) => { await twice(page); });\n'
                  'test("untagged", async ({ page }) => {\n'
                  '  await twice(page);\n'
                  '});\n'
                  'test("unrelated", async ({ page }) => { await page.goto("/"); });\n')
        self.assertEqual(scan(source), ['7:twice'])

    def test_helper_nested_in_a_describe_is_judged_where_it_is_called(self) -> None:
        source = ('test.describe("g", () => {\n'
                  '  async function rest(page) {\n'
                  '    await page.waitForTimeout(500);\n'
                  '  }\n'
                  '  const settle = (page) => rest(page);\n'
                  '  test("tagged", { tag: "@exclusive" }, async ({ page }) => { await settle(page); });\n'
                  '  test("untagged", async ({ page }) => {\n'
                  '    await settle(page);\n'
                  '  });\n'
                  '  test("unrelated", async ({ page }) => { await page.goto("/"); });\n'
                  '});\n')
        self.assertEqual(scan(source), ['8:settle'])

    def test_imported_helper_with_a_primitive_taints_callers(self) -> None:
        helper = 'export async function measure(page) {\n  return performance.now();\n}\n'
        source = ('import { measure } from "./timing";\n'
                  'test("a", async ({ page }) => {\n  await measure(page);\n});\n')
        self.assertEqual(scan(source, {'timing.ts': helper}), ['3:measure'])

    def test_untagged_hook_with_a_primitive_is_refused(self) -> None:
        source = ('test.beforeEach(async ({ page }) => {\n  await page.waitForTimeout(1);\n});\n'
                  'test("a", async () => {});\n')
        self.assertEqual(scan(source), ['2:waitForTimeout'])
        tagged = ('test.describe("g", { tag: "@exclusive" }, () => {\n'
                  '  test.beforeEach(async ({ page }) => { await page.waitForTimeout(1); });\n'
                  '  test("a", async () => {});\n});\n')
        self.assertEqual(scan(tagged), [])

    def test_module_level_primitive_outside_any_function_is_refused(self) -> None:
        self.assertEqual(scan('const started = Date.now();\ntest("a", async () => {});\n'),
                         ['1:Date.now'])

    def test_unbalanced_source_fails_closed(self) -> None:
        with self.assertRaises(ValueError):
            scan('test("a", async () => {\n  await page.waitForTimeout(1);\n')

    def test_an_empty_directory_is_not_a_pass(self) -> None:
        with tempfile.TemporaryDirectory() as directory, self.assertRaises(ValueError):
            lanes.check_directory(Path(directory))

    def test_the_repository_specs_pass_the_guard(self) -> None:
        findings = lanes.check_directory(ROOT / 'apps/knx-web/e2e')
        self.assertEqual([str(f) for f in findings], [])

    def test_the_parallel_lane_policy_file_is_gone(self) -> None:
        # The per-file hash admission was replaced by the tag guard (ADR-0144).
        self.assertFalse((ROOT / 'apps/knx-web/browser-lanes.json').exists())


if __name__ == '__main__':
    unittest.main()
