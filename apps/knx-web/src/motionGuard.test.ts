import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { parseRules } from "./themeTokens";

/**
 * The roadmap's standing rule: every animation this application ships must
 * be switchable off from inside the application. That rule has already
 * been broken once by accident (a `transition`/`animation` declaration
 * living outside the `prefers-reduced-motion: no-preference` guard, or
 * using a literal duration the motion control can't touch). This checker
 * makes breaking it again fail the test suite instead of shipping quietly.
 */
export interface MotionGuardFinding {
  /** 1-based line number of the offending declaration in the source text. */
  line: number;
  /** The offending declaration's own text (first line only). */
  text: string;
  reason: string;
}

const MOTION_PROPERTY = /^(transition|animation)\s*:/;
// Any bare numeric time literal (e.g. "200ms", "0.2s") outside of the one
// permitted custom property reference.
const LITERAL_DURATION = /(^|[^-\w])\d+(\.\d+)?m?s\b/;
const REDUCED_MOTION_MEDIA = /^@media\s*\(\s*prefers-reduced-motion\s*:\s*no-preference\s*\)$/;

/**
 * Reports every `transition:`/`animation:` declaration that is not both:
 *
 * 1. nested inside a `@media (prefers-reduced-motion: no-preference)` block, and
 * 2. driven by `var(--knx-transition-duration)` rather than a literal duration.
 *
 * The stylesheet walking is `themeTokens.ts`'s `parseRules`, not a second
 * hand-rolled brace counter. This file used to carry its own — the same
 * algorithm, comment stripping and all, differing only in that it tracked
 * the enclosing selector chain, which is what question 1 asks about.
 * `parseRules` now carries that chain as `rule.ancestors`, so there is one
 * CSS scanner in this app and both guards are wrong or right together.
 */
export function checkMotionDeclarations(css: string): MotionGuardFinding[] {
  const findings: MotionGuardFinding[] = [];
  for (const rule of parseRules(css)) {
    const insideNoPreference = [...rule.ancestors, rule.selector].some((header) =>
      REDUCED_MOTION_MEDIA.test(header),
    );
    for (const declaration of rule.declarations) {
      const stmt = `${declaration.property}: ${declaration.value}`;
      if (!MOTION_PROPERTY.test(stmt)) continue;
      const usesVar = stmt.includes("var(--knx-transition-duration)");
      const hasLiteral = LITERAL_DURATION.test(stmt);
      if (insideNoPreference && usesVar && !hasLiteral) continue;
      const reason = !insideNoPreference
        ? "not inside a @media (prefers-reduced-motion: no-preference) block"
        : hasLiteral
          ? "uses a literal duration instead of var(--knx-transition-duration)"
          : "does not drive its duration from var(--knx-transition-duration)";
      findings.push({ line: declaration.line, text: stmt, reason });
    }
  }
  return findings;
}

describe("checkMotionDeclarations", () => {
  it("flags a transition outside any media block", () => {
    const css = `.foo {\n  transition: transform var(--knx-transition-duration) ease;\n}\n`;
    const findings = checkMotionDeclarations(css);
    expect(findings).toHaveLength(1);
    expect(findings[0].reason).toMatch(/prefers-reduced-motion/);
  });

  it("flags a transition inside a no-preference block with a literal duration", () => {
    const css = `@media (prefers-reduced-motion: no-preference) {\n  .foo {\n    transition: transform 200ms ease;\n  }\n}\n`;
    const findings = checkMotionDeclarations(css);
    expect(findings).toHaveLength(1);
    expect(findings[0].reason).toMatch(/literal duration/);
  });

  // The ancestor chain, not just the immediate parent: the guard's own
  // question is whether *any* enclosing block is the reduced-motion media
  // query, and a rule set two levels down is the case that tells a chain
  // apart from a parent pointer.
  it("accepts a declaration guarded two levels up, and flags its unguarded sibling", () => {
    const css = [
      "@media (prefers-reduced-motion: no-preference) {",
      "  @supports (display: grid) {",
      "    .deep {",
      "      transition: transform var(--knx-transition-duration) var(--knx-motion-easing);",
      "    }",
      "  }",
      "}",
      "@supports (display: grid) {",
      "  .shallow {",
      "    transition: transform var(--knx-transition-duration) var(--knx-motion-easing);",
      "  }",
      "}",
    ].join("\n");
    const findings = checkMotionDeclarations(css);
    expect(findings).toHaveLength(1);
    expect(findings[0].text).toBe(
      "transition: transform var(--knx-transition-duration) var(--knx-motion-easing)",
    );
    expect(findings[0].line).toBe(10);
    expect(findings[0].reason).toMatch(/prefers-reduced-motion/);
  });

  // Shared blind spot, shared fix: the semicolon before `}` is optional,
  // and while `parseRules` flushed only on `;` this declaration was
  // invisible to the guard — a literal 200ms the motion control cannot
  // touch, inside the one block where the guard is supposed to be
  // strictest, passing quietly.
  it("flags a literal duration written without its final semicolon", () => {
    const css = `@media (prefers-reduced-motion: no-preference) {\n  .m {\n    transition: opacity 200ms ease\n  }\n}\n`;
    const findings = checkMotionDeclarations(css);
    expect(findings).toHaveLength(1);
    expect(findings[0].reason).toMatch(/literal duration/);
    expect(findings[0].line).toBe(3);
  });

  it("passes a fully compliant declaration", () => {
    const css = `@media (prefers-reduced-motion: no-preference) {\n  .foo {\n    transition: transform var(--knx-transition-duration) var(--knx-motion-easing);\n  }\n}\n`;
    expect(checkMotionDeclarations(css)).toEqual([]);
  });

  it("reports exactly the two non-compliant fixtures and none of the compliant one", () => {
    const css = [
      ".unguarded {",
      "  transition: transform var(--knx-transition-duration) ease;",
      "}",
      "@media (prefers-reduced-motion: no-preference) {",
      "  .literal {",
      "    transition: transform 200ms ease;",
      "  }",
      "  .compliant {",
      "    transition: transform var(--knx-transition-duration) var(--knx-motion-easing);",
      "  }",
      "}",
    ].join("\n");
    const findings = checkMotionDeclarations(css);
    expect(findings).toHaveLength(2);
  });
});

// One hard-coded path, on purpose, and it is the weakest thing in this
// file. `themeTokens.test.ts` walks for every `.css` file precisely
// because a second stylesheet would otherwise sit outside ADR-0022, and
// the same argument applies verbatim here: the roadmap's standing rule is
// that *every* animation this application ships is switchable off, and a
// second stylesheet would get boundary coverage from that walk and no
// motion coverage at all. `help.test.ts` and `diagnosticShell.test.ts`
// read the same single path for their own reasons. Recorded rather than
// built — the walk belongs in one shared helper, not in a third copy.
describe("styles.css", () => {
  it("has no unguarded transition/animation declarations", () => {
    const stylesheetPath = join(dirname(fileURLToPath(import.meta.url)), "styles.css");
    const css = readFileSync(stylesheetPath, "utf-8");
    const findings = checkMotionDeclarations(css);
    const details = findings
      .map((f) => `  line ${f.line}: ${f.text}\n    reason: ${f.reason}`)
      .join("\n");
    expect(
      findings,
      `styles.css has ${findings.length} unguarded motion declaration(s):\n${details}`,
    ).toEqual([]);
  });
});
