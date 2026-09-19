# T38 — Web version from the package manifest

## Root cause

The web manifest already carried `0.1.0-alpha.1`, but the visible shell did
not consume it. `App.tsx` rendered the fixed footer text `KNX-compatible`, and
`index.html` provided only the unversioned bootstrap title `KNXBench`.

## Design

- Import the named `version` field from `apps/knx-web/package.json` in the
  application module. Vite resolves this JSON import during the build, leaving
  `package.json` as the only application version source.
- Default the root `App` component's `manifestVersion` prop to that imported
  value. The prop is an explicit test seam: production callers provide
  nothing, while tests inject a sentinel version unrelated to the current
  manifest value.
- Render `v<manifestVersion>` in the right footer span and update
  `document.title` to `KNXBench <manifestVersion>` in an effect.
- Keep the existing `KNX-compatible · Linux-first` welcome copy unchanged.

## TDD evidence

Before production changes, two `App.test.tsx` tests injected
`98.76.54-test`. The focused run had 18 existing passes and exactly two
expected failures:

- footer: expected `v98.76.54-test`, received `KNX-compatible`
- title: expected `KNXBench 98.76.54-test`, received an empty title

After the implementation, the focused file passed 21/21 tests. Two safeguards
cover the complete path: the literal sentinel means a fixed or stale
application version cannot satisfy the injected path, while a default render
asserts both outputs against the imported `package.json` version and protects
the production wiring.

## Verification

Run from `apps/knx-web`:

```text
npm test -- --run src/App.test.tsx
# 1 file passed; 21 tests passed

npx tsc -p tsconfig.json --noEmit
# exit 0

npm test
# 42 files passed; 471 tests passed

npm run build
# tsc passed; Vite transformed 81 modules and completed the production build

test -z "$(grep -R -l --include='*.js' 'package.json' dist/assets || true)"
# exit 0: no runtime package.json reference in the JavaScript bundle

grep -R -l --include='*.js' '0.1.0-alpha.1' dist/assets
# the production JavaScript bundle contains the resolved manifest version
```

`npm run build` clears `dist`; the tracked `dist/.gitkeep` was restored after
the successful build. No Rust or architecture boundary changed.
