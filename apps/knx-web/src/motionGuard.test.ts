import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";

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
 * Walks stylesheet text by brace counting (no CSS parser) and reports every
 * `transition:`/`animation:` declaration that is not both:
 *
 * 1. nested inside a `@media (prefers-reduced-motion: no-preference)` block, and
 * 2. driven by `var(--knx-transition-duration)` rather than a literal duration.
 */
export function checkMotionDeclarations(css: string): MotionGuardFinding[] {
  // Blank out comment bodies (keep newlines) so braces/semicolons inside
  // comments can't confuse the brace counter.
  const stripped = css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));

  const findings: MotionGuardFinding[] = [];
  const blockStack: string[] = [];
  let line = 1;
  let buffer = "";
  let bufferStartLine = 1;
  let bufferHasContent = false;

  const flushDeclaration = () => {
    const stmt = buffer.trim();
    if (MOTION_PROPERTY.test(stmt)) {
      const insideNoPreference = blockStack.some((header) => REDUCED_MOTION_MEDIA.test(header));
      const usesVar = stmt.includes("var(--knx-transition-duration)");
      const hasLiteral = LITERAL_DURATION.test(stmt);
      if (!insideNoPreference || !usesVar || hasLiteral) {
        const reason = !insideNoPreference
          ? "not inside a @media (prefers-reduced-motion: no-preference) block"
          : hasLiteral
            ? "uses a literal duration instead of var(--knx-transition-duration)"
            : "does not drive its duration from var(--knx-transition-duration)";
        findings.push({ line: bufferStartLine, text: stmt.split("\n")[0].trim(), reason });
      }
    }
  };

  for (let i = 0; i < stripped.length; i++) {
    const ch = stripped[i];
    if (ch === "\n") {
      line++;
    }
    if (ch === "{") {
      blockStack.push(buffer.trim().replace(/\s+/g, " "));
      buffer = "";
      bufferHasContent = false;
      continue;
    }
    if (ch === "}") {
      blockStack.pop();
      buffer = "";
      bufferHasContent = false;
      continue;
    }
    if (ch === ";") {
      flushDeclaration();
      buffer = "";
      bufferHasContent = false;
      continue;
    }
    if (!bufferHasContent && ch.trim() !== "") {
      bufferStartLine = line;
      bufferHasContent = true;
    }
    buffer += ch;
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
