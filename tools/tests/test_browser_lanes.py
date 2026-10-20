"""Browser lane guard: real-time measurements must carry the @exclusive tag."""
from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from tools import browser_lanes as lanes

ROOT = Path(__file__).resolve().parents[2]


def scan(source: str, helpers: dict[str, str] | None = None) -> list[str]:
    """Return 'line:primitive' for spec findings and 'file:line:primitive' for helper findings."""
    with tempfile.TemporaryDirectory() as directory:
        e2e = Path(directory)
        (e2e / 'spec.e2e.ts').write_text(source)
        for name, text in (helpers or {}).items():
            (e2e / name).parent.mkdir(parents=True, exist_ok=True)
            (e2e / name).write_text(text)
        return [f'{f.line}:{f.primitive}' if f.file == 'spec.e2e.ts' else f'{f.file}:{f.line}:{f.primitive}'
                for f in lanes.check_directory(e2e)]


UNTAGGED_CALL = 'test("a", async ({ page }) => {\n  await %s;\n});\n'


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

    def test_a_type_reference_is_not_a_use(self) -> None:
        helper = 'interface Clock {\n  timer?: ReturnType<typeof setTimeout>;\n}\nexport const MAX = 3;\n'
        source = ('import { MAX } from "./clock";\n'
                  'test("a", async () => {\n  let t: ReturnType<typeof setInterval> | undefined;\n});\n')
        self.assertEqual(scan(source, {'clock.ts': helper}), [])

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

    def test_a_fixture_that_waits_is_refused_where_it_is_declared(self) -> None:
        helper = ('import { test as base } from "@playwright/test";\n'
                  'export const test = base.extend<{ settled: void }>({\n'
                  '  settled: async ({ page }, use) => {\n'
                  '    await page.waitForTimeout(300);\n'
                  '    await use();\n'
                  '  },\n'
                  '});\n')
        source = 'import { test } from "./fixtures";\ntest("a", async ({ settled }) => {});\n'
        self.assertEqual(scan(source, {'fixtures.ts': helper}), ['fixtures.ts:4:waitForTimeout'])

    def test_a_helper_primitive_outside_any_function_is_refused_in_the_helper(self) -> None:
        source = 'import { started } from "./clock";\ntest("a", async () => {});\n'
        self.assertEqual(scan(source, {'clock.ts': 'export const started = Date.now();\n'}),
                         ['clock.ts:1:Date.now'])

    def test_object_and_class_methods_taint_their_member_calls(self) -> None:
        helper = ('export const h = {\n  async settle(page) {\n    await page.waitForTimeout(1);\n  },\n'
                  '  pause: async (page) => { await page.waitForTimeout(1); },\n};\n'
                  'export class Clock {\n  async rest(page) {\n    await page.waitForTimeout(1);\n  }\n}\n')
        source = ('import { h, Clock } from "./h";\n'
                  'test("a", async ({ page }) => {\n'
                  '  await h.settle(page);\n'
                  '  await h.pause(page);\n'
                  '  await new Clock().rest(page);\n'
                  '  await page.click("x");\n'
                  '});\n')
        self.assertEqual(scan(source, {'h.ts': helper}), ['3:settle', '4:pause', '5:rest'])

    def test_every_import_shape_is_followed(self) -> None:
        timing = 'export async function measure(page) {\n  return performance.now();\n}\n'
        helpers = {
            'timing.ts': timing,
            'settle.ts': 'export default async function settle(page) {\n  await page.waitForTimeout(1);\n}\n',
            'helpers/impl.ts': 'export async function sleep(page) {\n  await page.waitForTimeout(1);\n}\n',
            'helpers/index.ts': 'export { sleep } from "./impl";\nexport * from "./impl";\n',
        }
        cases = {
            'import { measure } from "./timing.js";\n': 'measure(page)',
            'import * as t from "./timing";\n': 't.measure(page)',
            'import wait from "./settle";\n': 'wait(page)',
            'import { sleep } from "./helpers";\n': 'sleep(page)',
            'import { sleep as nap } from "./helpers/index";\n': 'nap(page)',
        }
        for header, call in cases.items():
            self.assertEqual(len(scan(header + UNTAGGED_CALL % call, helpers)), 1, header)

    def test_same_file_object_and_class_methods_taint_their_member_calls(self) -> None:
        source = ('const h = {\n  async wait(page) { await page.waitForTimeout(1); },\n'
                  '  pause: async (page) => { await page.waitForTimeout(1); },\n};\n'
                  'class Clock {\n  async rest(page) { await page.waitForTimeout(1); }\n'
                  '  async twice(page) { await this.rest(page); }\n}\n'
                  'test("a", async ({ page }) => {\n'
                  '  await h.wait(page);\n'
                  '  await h.pause(page);\n'
                  '  await new Clock().twice(page);\n'
                  '});\n')
        self.assertEqual(scan(source), ['10:wait', '11:pause', '12:twice'])

    def test_a_function_declaration_is_not_a_method_of_every_object(self) -> None:
        source = ('async function press(page) {\n  await page.waitForTimeout(5);\n}\n'
                  'test("tagged", { tag: "@exclusive" }, async ({ page }) => { await press(page); });\n'
                  'test("a", async ({ page }) => {\n'
                  '  await page.keyboard.press("Enter");\n'
                  '  await page.locator("x").press("Tab");\n'
                  '});\n')
        self.assertEqual(scan(source), [])
        helper = ('async function press(page) {\n  await page.waitForTimeout(5);\n}\n'
                  'export async function login(page) {\n  await page.keyboard.press("Enter");\n}\n')
        self.assertEqual(scan('import { login } from "./auth";\n' + UNTAGGED_CALL % 'login(page)',
                              {'auth.ts': helper}), [])

    def test_more_import_and_export_shapes_are_followed(self) -> None:
        impl = 'export async function nap(page) {\n  await page.waitForTimeout(1);\n}\n'
        helpers = {
            'impl.ts': impl,
            'deflt.ts': 'export default async function snooze(page) {\n  await page.waitForTimeout(1);\n}\n',
            'ns.ts': 'export * as timing from "./impl";\n',
            'named.ts': 'export { default as snooze } from "./deflt";\n',
        }
        cases = {
            'import { timing } from "./ns";\n': 'timing.nap(page)',
            'import { snooze } from "./named";\n': 'snooze(page)',
            'const { nap } = await import("./impl");\n': 'nap(page)',
            'import { default as snooze } from "./deflt";\n': 'snooze(page)',
        }
        for header, call in cases.items():
            self.assertEqual(len(scan(header + UNTAGGED_CALL % call, helpers)), 1, header)

    def test_a_side_effect_import_that_registers_a_waiting_hook_is_refused(self) -> None:
        setup = 'import { test } from "@playwright/test";\ntest.beforeEach(async ({ page }) => {\n  await page.waitForTimeout(1);\n});\n'
        self.assertEqual(scan('import "./setup";\ntest("a", async () => {});\n', {'setup.ts': setup}),
                         ['setup.ts:3:waitForTimeout'])

    def test_a_js_specifier_for_a_tsx_module_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            scan('import { Page } from "./page.js";\ntest("a", async () => {});\n',
                 {'page.tsx': 'export const Page = () => <div />;\n'})

    def test_import_cycles_are_judged_the_same_in_any_order(self) -> None:
        helpers = {
            'b.ts': 'import { c } from "./c";\nexport async function b(page) {\n  await c(page);\n}\n',
            'c.ts': ('import { b } from "./b";\nexport async function c(page) {\n  await page.waitForTimeout(1);\n}\n'
                     'export async function d(page) {\n  await b(page);\n}\n'),
        }
        for header in ('import { b } from "./b";\nimport { c, d } from "./c";\n',
                       'import { c, d } from "./c";\nimport { b } from "./b";\n'):
            self.assertEqual(len(scan(header + UNTAGGED_CALL % 'b(page)', helpers)), 1, header)

    def test_app_modules_outside_the_browser_tests_are_not_followed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            (app / 'e2e').mkdir()
            (app / 'src').mkdir()
            (app / 'src/toast.ts').write_text('export function add(x) {\n  return setTimeout(() => x, 5);\n}\n')
            (app / 'e2e/spec.e2e.ts').write_text(
                'import { add } from "../src/toast";\n'
                'test("a", async () => {\n  const seen = new Set();\n  seen.add(1);\n});\n')
            self.assertEqual(lanes.check_directory(app / 'e2e'), [])

    def test_timer_forms_beyond_the_globals_count(self) -> None:
        source = ('import { setTimeout as sleep } from "node:timers/promises";\n'
                  'test("tagged", { tag: "@exclusive" }, async () => {\n  await sleep(5);\n});\n'
                  'test("a", async () => {\n'
                  '  await sleep(5);\n'
                  '  await scheduler.wait(5);\n'
                  '  const kind = typeof Date.now();\n'
                  '});\n')
        self.assertEqual(scan(source), ['6:sleep', '7:scheduler.wait', '8:Date.now'])

    def test_an_imported_tsx_module_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            scan('import { Page } from "./page";\ntest("a", async () => {});\n',
                 {'page.tsx': 'export const Page = () => <div />;\n'})

    def test_qualified_clocks_and_date_arithmetic_count(self) -> None:
        source = ('test("a", async () => {\n'
                  '  const a = window.performance.now();\n'
                  '  const b = globalThis.Date.now();\n'
                  '  const c = new Date().getTime();\n'
                  '  const stamp = new Date().toISOString();\n'
                  '  const fixed = new Date("2026-01-01").getTime();\n'
                  '});\n')
        self.assertEqual(scan(source), ['2:performance.now', '3:Date.now', '4:new Date().getTime'])

    def test_code_passed_to_the_page_as_a_string_counts(self) -> None:
        source = ('test("a setTimeout( title", async ({ page }) => {\n'
                  '  await page.evaluate("new Promise((r) => setTimeout(r, 50))");\n'
                  '  await page.addInitScript(`window.tick = setInterval(() => {}, 5)`);\n'
                  '  await page.fill("#x", "setTimeout(1)");\n'
                  '});\n')
        self.assertEqual(scan(source), ['2:setTimeout', '3:setInterval'])

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


class LiteralBracketClockTests(unittest.TestCase):
    @staticmethod
    def cases() -> list[tuple[str, str]]:
        return [(form.format(clock=clock, quote=quote), f'{clock}[{quote}now{quote}]')
                for clock in ('Date', 'performance') for quote in ('"', "'")
                for form in ('{clock}[{quote}now{quote}]()',
                             '{clock} [ {quote}now{quote} ] ( )',
                             '{clock}\n [\n {quote}now{quote}\n ]\n (\n )')]

    def test_literal_bracket_clocks_refuse_untagged_calls(self) -> None:
        for expression, primitive in self.cases():
            with self.subTest(expression=expression):
                source = 'test("a", async () => {\n  const t = ' + expression + ';\n});\n'
                self.assertEqual(scan(source), [f'2:{primitive}'])

    def test_literal_bracket_clocks_in_page_function_callbacks_are_refused(self) -> None:
        for expression, primitive in self.cases():
            for callback in ('() => ', 'function () { return '):
                with self.subTest(expression=expression, callback=callback):
                    body = callback + expression + ('; }' if callback.startswith('function') else '')
                    source = 'test("a", async ({ page }) => {\n  await page.evaluate(' + body + ');\n});\n'
                    self.assertEqual(scan(source), [f'2:{primitive}'])

    def test_literal_bracket_clocks_admit_tagged_tests(self) -> None:
        for expression, _ in self.cases():
            with self.subTest(expression=expression):
                self.assertEqual(scan('test("a", { tag: "@exclusive" }, async () => {\n'
                                      '  const t = ' + expression + ';\n});\n'), [])

    def test_literal_bracket_clocks_admit_enclosing_tagged_describes(self) -> None:
        for expression, _ in self.cases():
            with self.subTest(expression=expression):
                self.assertEqual(scan('test.describe("g", { tag: ["@slow", "@exclusive"] }, () => {\n'
                                      '  test("a", async ({ page }) => {\n'
                                      '    await page.evaluate(() => ' + expression + ');\n'
                                      '  });\n});\n'), [])

    def test_literal_bracket_clocks_taint_transitive_local_helpers(self) -> None:
        for expression, _ in self.cases():
            with self.subTest(expression=expression):
                source = ('function clock() { return ' + expression + '; }\n'
                          'const measure = () => clock();\n'
                          'test("tagged", { tag: "@exclusive" }, async () => { measure(); });\n'
                          'test("untagged", async () => {\n  measure();\n});\n'
                          'test("unrelated", async () => { Math.abs(-1); });\n')
                call_line = source.splitlines().index('  measure();') + 1
                self.assertEqual(scan(source), [f'{call_line}:measure'])

    def test_literal_bracket_clocks_taint_imported_helpers_not_unrelated_callers(self) -> None:
        for expression, _ in self.cases():
            with self.subTest(expression=expression):
                helpers = {
                    'clock.ts': ('export function clock() { return ' + expression + '; }\n'
                                 'export function safe() { return 1; }\n'),
                    'timing.ts': ('import { clock } from "./clock";\n'
                                  'export const measure = () => clock();\n'
                                  'export { safe } from "./clock";\n'),
                }
                header = 'import { measure as sample, safe } from "./timing";\n'
                admitted = ('test("tagged", { tag: "@exclusive" }, async () => { sample(); });\n'
                            'test.describe("g", { tag: "@exclusive" }, () => {\n'
                            '  test("nested", async () => { sample(); });\n});\n'
                            'test("unrelated", async () => { safe(); });\n')
                self.assertEqual(scan(header + admitted, helpers), [])
                source = header + admitted + 'test("untagged", async () => {\n  sample();\n});\n'
                self.assertEqual(scan(source, helpers), ['8:sample'])

    def test_literal_bracket_clock_text_is_masked(self) -> None:
        for clock in ('Date', 'performance'):
            for quote, outer in (('"', "'"), ("'", '"')):
                expression = f'{clock}[{quote}now{quote}]()'
                texts = [f'// {expression}\n', f'/* {expression} */\n',
                         f'test({outer}{expression}{outer}, async () => {{}});\n',
                         f'const text = {outer}page.evaluate(() => {expression}){outer};\n',
                         f'const pattern = /{expression}/;\n',
                         f'const template = `page.evaluate(() => {expression})`;\n']
                for text in texts:
                    with self.subTest(text=text):
                        self.assertEqual(scan(text + 'test("a", async () => {});\n'), [])

    def test_literal_bracket_clocks_in_template_expressions_are_refused(self) -> None:
        for expression, primitive in self.cases():
            with self.subTest(expression=expression):
                source = ('test("a", async () => {\n'
                          '  const text = `clock: ${' + expression + '} trailing Date["now"]()`;\n});\n')
                self.assertEqual(scan(source), [f'2:{primitive}'])

    def test_literal_bracket_clock_diagnostics_preserve_source_lines(self) -> None:
        source = ('// Date["now"]()\n'
                  'test("performance[\'now\']()", async ({ page }) => {\n'
                  '  const text = `Date["now"]()\nmore text`;\n'
                  '  const a = Date\n [ "now" ]\n ();\n'
                  '  await page.evaluate(() => performance [ \'now\' ] ());\n'
                  '  const b = Date . now();\n});\n')
        with tempfile.TemporaryDirectory() as directory:
            e2e = Path(directory)
            (e2e / 'spec.e2e.ts').write_text(source)
            self.assertEqual([str(f) for f in lanes.check_directory(e2e)], [
                'spec.e2e.ts:5: Date["now"] measures real time outside an @exclusive scope; '
                'tag the test or its describe with { tag: "@exclusive" }',
                "spec.e2e.ts:8: performance['now'] measures real time outside an @exclusive scope; "
                'tag the test or its describe with { tag: "@exclusive" }',
                'spec.e2e.ts:9: Date.now measures real time outside an @exclusive scope; '
                'tag the test or its describe with { tag: "@exclusive" }',
            ])

    def test_literal_bracket_clock_module_use_is_refused_in_imported_helper(self) -> None:
        source = 'import { started } from "./clock";\ntest("a", async () => {});\n'
        for expression, primitive in self.cases():
            with self.subTest(expression=expression):
                self.assertEqual(scan(source, {'clock.ts': '\nexport const started = ' + expression + ';\n'}),
                                 [f'clock.ts:2:{primitive}'])

    def test_literal_bracket_clock_malformed_source_still_fails_closed(self) -> None:
        cases = [
            ('test("a", () => { Date["now"(); });', 'unbalanced bracket at offset'),
            ('test("a", () => { Date["now"]();', 'unclosed bracket at offset'),
            ('test("a", () => { Date["now\n](); });', 'unterminated string literal'),
            ('test("a", () => { performance[\'now\n](); });', 'unterminated string literal'),
            ('test("a", () => { Date["now"](); /*', 'unterminated block comment'),
            ('test("a", () => { const t = `${Date["now"]()} trailing', 'unterminated template literal'),
            ('test("a", () => { const r = /Date["now"]()\n', 'unterminated regular expression'),
        ]
        for source, message in cases:
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, message):
                scan(source)

    def test_computed_and_escaped_bracket_properties_are_not_literal_clocks(self) -> None:
        source = (r'''test("a", async () => {
  const a = Date["n\ow"]();
  const b = performance['n\x6fw']();
  const c = Date["no" + "w"]();
  const d = performance[key]();
  const e = Date[`now`]();
  const f = Date["now"];
  const g = otherDate["now"]();
  const h = $performance['now']();
});
''')
        self.assertEqual(scan(source), [])


if __name__ == '__main__':
    unittest.main()
