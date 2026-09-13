// Minimal ambient declarations for the handful of Node builtins used by
// test files that read project sources from disk (`motionGuard.test.ts`,
// and `DiagnosticsCompanion.test.tsx`, which walks an import graph and so
// also needs to ask whether a resolved path exists).
//
// Why this file exists instead of `@types/node`: this package's build is
// `tsc && vite build` with `include: ["src"]`, so the test files are type
// checked alongside the application, and a Node import with no declaration
// fails the production build even though Vitest runs it happily. T27's
// design forbids adding an npm dependency, and pulling the whole Node type
// surface into a browser application's compilation would also let a
// component import `node:fs` without anyone noticing. Declaring exactly
// what is used keeps both doors shut.
//
// If `@types/node` is ever added for another reason, delete this file —
// the real declarations are better than these.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf-8" | "utf8"): string;
  export function existsSync(path: string): boolean;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
}

declare module "node:path" {
  export function dirname(path: string): string;
  export function join(...paths: string[]): string;
}
