/** Allocate private, socket-budgeted browser temp before loading Playwright. */
const { execFileSync } = require("node:child_process");
const { rmSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");

let directory;
function ensureBrowserTmp() {
  if (directory) return directory;
  // Resolve relative to this module, not the caller's cwd or a baked worktree.
  const selector = join(__dirname, "browser_tmp.py");
  const parentOwned = process.env.KNXBENCH_BROWSER_TMP_DIR;
  const args = [selector, "--base", parentOwned || tmpdir(), "--allocate"];
  if (parentOwned) args.push("--reuse-owned");
  const response = JSON.parse(execFileSync("python3", args, { encoding: "utf8" }));
  if (typeof response?.directory !== "string" || typeof response.created !== "boolean") {
    throw new Error("Invalid browser temp allocation response");
  }
  directory = response.directory;
  process.env.TMPDIR = directory;
  // Subprocess config loaders reuse this parent-owned leaf; no second component.
  process.env.KNXBENCH_BROWSER_TMP_DIR = directory;
  if (response.created) {
    const owned = directory;
    process.once("exit", () => rmSync(owned, { recursive: true, force: true }));
  }
  return directory;
}

module.exports = { ensureBrowserTmp };
ensureBrowserTmp();
