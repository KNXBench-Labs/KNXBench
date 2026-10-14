/** Gives every Playwright entry a short, owned Chromium temporary directory. */
/// <reference types="node" />
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

let ownedDirectory: string | undefined;

export function ensureBrowserTmp(): string {
  if (ownedDirectory) return ownedDirectory;
  // One Python selector owns the path budget/approved-ancestor policy for both
  // this direct-Playwright entry and the measured Python gate.
  // Resolve the selected workspace at runtime; CJS and ESM config loaders both
  // work without import.meta or a baked checkout path.
  let root = process.env.KNXBENCH_TEST_ROOT ?? process.cwd();
  while (!existsSync(join(root, "tools/browser_tmp.py")) || !existsSync(join(root, "apps/knx-web/package-lock.json"))) {
    const parent = dirname(root);
    if (parent === root) throw new Error("Run Playwright from the KNXBench workspace or set KNXBENCH_TEST_ROOT");
    root = parent;
  }
  const selector = join(root, "tools/browser_tmp.py");
  const parentOwned = process.env.KNXBENCH_BROWSER_TMP_DIR;
  const args = parentOwned ? [selector, "--base", parentOwned, "--reuse-owned"]
    : [selector, "--base", tmpdir()];
  const shortRoot: unknown = JSON.parse(execFileSync("python3", args, { encoding: "utf8" }));
  if (typeof shortRoot !== "string") throw new Error("Invalid browser temp-root response");
  if (parentOwned) {
    // The gate's context manager owns cleanup, including signal cancellation.
    ownedDirectory = shortRoot;
    process.env.TMPDIR = shortRoot;
    return shortRoot;
  }
  ownedDirectory = mkdtempSync(join(shortRoot, "w"));
  const directory = ownedDirectory;
  process.env.TMPDIR = directory;
  process.once("exit", () => rmSync(directory, { recursive: true, force: true }));
  return directory;
}

ensureBrowserTmp();
