/** Panel rendering and editing a device's application-program parameters, fetched per selection. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type {
  ModuleScope,
  ParameterDiagnostic,
  ParameterDiagnosticKind,
  ParameterField,
  ParameterPanel as ParameterPanelDto,
  ParameterSection,
  StaleParameter,
} from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useProductLanguage } from "./productLanguage";
import { useTranslate, type Translate, type TranslatableKey } from "./i18n";

// T18 slice 3, task 4 (design docs/superpowers/specs/2026-09-11-parameter-editor-design.md).
// Fetches `GET /api/device/{id}/parameters` on every device selection —
// unconditionally, not gated on a `program_id` the client cannot see
// (`DeviceDetail` carries no such field): a device whose program does not
// resolve still returns 200 with `programId: null`, an empty `sections`,
// and — critically — that device's own `stale` entries (D21), which a
// conditional fetch would have hidden exactly where they matter most.

// D23's own fallback: `module_id` when present, else `"Module #{module_node}"`.
// Takes `t` rather than calling `useTranslate()` itself — this isn't a
// component, so it has no hook rules to follow, but it also has no render
// of its own to re-run on a language switch; the caller's `t` (from its own
// `useTranslate()`) is what makes this re-derive correctly.
function sectionLabel(t: Translate, scope: ModuleScope | null): string {
  if (scope === null) return t("parameters.deviceScope");
  return scope.moduleId ?? t("parameters.moduleNumber", { number: scope.moduleNode });
}

// One editable/disabled field row. Mirrors `Inspector.tsx`'s
// `ComObjectDescriptionField`/`DptField`: local draft state, commit on
// blur, revert and surface the error inline on a rejected write — applied
// here to `api.setParameterValue` instead of one of the ProjectTree
// commands. That endpoint's response rides the same DTO as the `GET`, but
// a successful write also carries the server's own freshly rebuilt `tree`
// (T3 fix round 1, item 6) — `onValueApplied` (threaded from `ParameterPanel`,
// see its own doc comment) hands that tree straight to the caller, so
// `App.tsx` can `setTree` it without reconstructing anything by hand. A
// module-scoped field is writable exactly when the server names a
// `writeEtsId` for it (design D43, superseding D25's blanket read-only
// rule) — that happens when exactly one imported module instance is its
// authority; every other case stays disabled with a caption naming the
// read-only reason.
function ParameterFieldRow(props: {
  field: ParameterField;
  deviceId: number;
  language: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
  onValueApplied: (tree: ProjectTree) => void;
}) {
  const { field, deviceId, language, onUpdated, onValueApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(field.value ?? "");
  const [error, setError] = useState<string | null>(null);
  // Belt-and-braces on purpose (see `api.ts`'s `writeEtsId` doc comment):
  // the server's contract makes the two conditions exact opposites of
  // each other, but the control checks both rather than trusting either
  // one alone.
  const disabled = !field.editable || field.writeEtsId === null;

  useEffect(() => {
    setValue(field.value ?? "");
    setError(null);
  }, [field.etsId, field.value]);

  async function apply() {
    if (!field.editable || field.writeEtsId === null) return;
    const current = field.value ?? "";
    if (value === current) return;
    setError(null);
    try {
      const panel = await api.setParameterValue(deviceId, field.writeEtsId, value, language);
      onUpdated(panel);
      // `set_parameter_value_impl` always attaches its own freshly rebuilt
      // tree to a successful write (T3 fix round 1, item 6) — a `null` here
      // would mean the server broke that contract, not that there is
      // nothing to publish, so this fails loudly rather than swallowing a
      // parameter edit's republish the way the pre-fix code silently did.
      if (!panel.tree) {
        throw new Error("setParameterValue response carried no tree");
      }
      onValueApplied(panel.tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  // `text` before `name`, not the other way round: measured on the real
  // corpus, `Name` has zero rows in the translation table for `parameter`
  // while `Text` has 9915, and every one of the 3557 corpus parameter rows
  // populates both columns (typically `name = "General"`, `text =
  // "Allgemein"`). `name ?? text` would make every translated string
  // unreachable behind the untranslated one that is always present — the
  // whole point of this slice. Do not "fix" this back.
  const label = field.text ?? field.name ?? field.etsId;

  return (
    <label className="inspector-field parameter-field">
      {label}
      {field.access && <span className="provenance-badge">{field.access}</span>}
      {field.kind === "Restriction" ? (
        <select
          value={value}
          disabled={disabled}
          onChange={(e) => setValue(e.target.value)}
          onBlur={apply}
        >
          <option value="">{t("parameters.none")}</option>
          {field.enumOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.text ?? opt.value}
            </option>
          ))}
        </select>
      ) : field.kind === "Number" || field.kind === "Time" ? (
        <input
          type="number"
          value={value}
          min={field.min ?? undefined}
          max={field.max ?? undefined}
          disabled={disabled}
          onChange={(e) => setValue(e.target.value)}
          onBlur={apply}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      ) : (
        <input
          type="text"
          value={value}
          disabled={disabled}
          onChange={(e) => setValue(e.target.value)}
          onBlur={apply}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      )}
      {disabled && (
        <span className="parameter-field-caption">
          {t("parameters.sharedReadOnlyCaption")}
        </span>
      )}
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

// `ParameterDiagnostic.scope` is built server-side from the very same
// section's own scope (`module_scope_dto`), so comparing field by field
// — rather than reference or `JSON.stringify` — is enough to find a
// section's own diagnostics.
function sameScope(a: ModuleScope | null, b: ModuleScope | null): boolean {
  if (a === null || b === null) return a === b;
  return a.moduleNode === b.moduleNode && a.moduleId === b.moduleId && a.moduleDefId === b.moduleDefId;
}

// KNOWN_LIMITATIONS.md §66: `ParameterDiagnostic.message` used to be
// server-composed English prose rendered verbatim regardless of UI
// language. `ParameterDiagnosticKindDto` (`apps/knx-server/src/routes.rs`)
// gives every one of the nineteen possible messages a closed, stable tag
// with no dynamic payload of its own (every id/count they'd want to name
// lives in `.detail` instead) — so, unlike `CatalogBrowser.tsx`'s
// `describeCreationDiagnostic`, this is a plain lookup, not a
// per-`kind` sentence assembled from structured fields. `.detail` stays
// untouched by this function on purpose: it is always English, by design
// (see `ParameterDiagnostic.detail`'s own doc comment in `api.ts`).
const PARAMETER_DIAGNOSTIC_MESSAGE_KEYS: Record<ParameterDiagnosticKind, TranslatableKey> = {
  parametersUnreadable: "parameters.diagnostic.parametersUnreadable",
  duplicateUnscopedValue: "parameters.diagnostic.duplicateUnscopedValue",
  duplicateModuleScopedValue: "parameters.diagnostic.duplicateModuleScopedValue",
  duplicateModuleId: "parameters.diagnostic.duplicateModuleId",
  noModuleInstanceMatch: "parameters.diagnostic.noModuleInstanceMatch",
  ambiguousModuleInstance: "parameters.diagnostic.ambiguousModuleInstance",
  malformedModuleInstanceId: "parameters.diagnostic.malformedModuleInstanceId",
  noBranchMatched: "parameters.diagnostic.noBranchMatched",
  unparsableTest: "parameters.diagnostic.unparsableTest",
  unresolvedParamRef: "parameters.diagnostic.unresolvedParamRef",
  nonNumericValue: "parameters.diagnostic.nonNumericValue",
  unexpectedTypeNoneShape: "parameters.diagnostic.unexpectedTypeNoneShape",
  unrecognizedNode: "parameters.diagnostic.unrecognizedNode",
  refBelowSkippedNode: "parameters.diagnostic.refBelowSkippedNode",
  moduleDefNotFound: "parameters.diagnostic.moduleDefNotFound",
  moduleCycleDetected: "parameters.diagnostic.moduleCycleDetected",
  moduleNestingTooDeep: "parameters.diagnostic.moduleNestingTooDeep",
  moduleExpansionBudgetExhausted: "parameters.diagnostic.moduleExpansionBudgetExhausted",
  missingValue: "parameters.diagnostic.missingValue",
  moduleWithoutId: "parameters.diagnostic.moduleWithoutId",
  moduleArgumentNotBound: "parameters.diagnostic.moduleArgumentNotBound",
  unsupportedModuleArgumentKind: "parameters.diagnostic.unsupportedModuleArgumentKind",
  unresolvedTextPlaceholder: "parameters.diagnostic.unresolvedTextPlaceholder",
};

function describeParameterDiagnosticMessage(t: Translate, diagnostic: ParameterDiagnostic): string {
  const key = PARAMETER_DIAGNOSTIC_MESSAGE_KEYS[diagnostic.kind];
  return key ? t(key) : diagnostic.message;
}

// One collapsible group per `ParameterSectionDto` — the top-level
// (`scope: null`) section and one per module instantiation (D23: "12
// instantiations are 12 results," never collapsed back into one list).
// Its own diagnostics (design D43) render inline, right below the
// summary — the same `parameters.*` text `DiagnosticsBanner` already
// shows panel-wide, surfaced again here so a section's read-only reason
// doesn't require hunting through that collapsed, unfiltered list.
function ParameterSectionView(props: {
  section: ParameterSection;
  diagnostics: ParameterDiagnostic[];
  deviceId: number;
  language: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
  onValueApplied: (tree: ProjectTree) => void;
}) {
  const { section, diagnostics, deviceId, language, onUpdated, onValueApplied } = props;
  const t = useTranslate();
  const ownDiagnostics = diagnostics.filter((d) => sameScope(d.scope, section.scope));
  return (
    <details className="parameter-section" open>
      <summary>{sectionLabel(t, section.scope)}</summary>
      {/* KNOWN_LIMITATIONS.md §66: the headline is translated via
          `d.kind`; `d.detail` (not shown here at all) stays English by
          design — see `ParameterDiagnostic.detail`'s doc comment. */}
      {ownDiagnostics.map((d, i) => (
        <p key={i} className="inspector-description">
          {describeParameterDiagnosticMessage(t, d)}
        </p>
      ))}
      <div className="parameter-fields">
        {section.fields.map((field) => (
          <ParameterFieldRow
            key={field.etsId}
            field={field}
            deviceId={deviceId}
            language={language}
            onUpdated={onUpdated}
            onValueApplied={onValueApplied}
          />
        ))}
      </div>
    </details>
  );
}

// D21's stored-but-unmatched values: shown separately from the normal
// field list, never merged in and never hidden.
function StaleParametersSection(props: { stale: StaleParameter[] }) {
  const { stale } = props;
  const t = useTranslate();
  return (
    <details className="parameter-stale-section" open>
      <summary>{t("parameters.staleValuesHeading", { count: stale.length })}</summary>
      <p className="inspector-description">{t("parameters.staleDescription")}</p>
      <ul>
        {stale.map((s) => (
          <li key={s.etsId} className="parameter-stale-entry">
            <span className="parameter-stale-ets-id">{s.etsId}</span>: {s.raw}
          </li>
        ))}
      </ul>
    </details>
  );
}

// D26's collapsed, count-headed banner — never a debugger. `detail` sits
// behind a "copy details" affordance, not printed inline. An unknown
// severity falls back to warning, never to a silent informational label.
function DiagnosticsBanner(props: { diagnostics: ParameterDiagnostic[] }) {
  const { diagnostics } = props;
  const t = useTranslate();
  const notes = diagnostics.filter((d) => d.severity === "info").length;
  const warnings = diagnostics.length - notes;
  const summary = [
    warnings > 0 ? t(warnings === 1 ? "parameters.warningsCount.one" : "parameters.warningsCount.other", { count: warnings }) : null,
    notes > 0 ? t(notes === 1 ? "parameters.notesCount.one" : "parameters.notesCount.other", { count: notes }) : null,
  ].filter((text) => text !== null).join(" · ");

  function copyDetail(detail: string) {
    void navigator.clipboard?.writeText(detail);
  }

  return (
    <details className="parameter-diagnostics-banner" data-has-warnings={warnings > 0}>
      <summary>{summary}</summary>
      <ul>
        {diagnostics.map((d, i) => {
          const severity = d.severity === "info" ? "info" : "warning";
          return <li key={i} data-severity={severity}>
            <strong>{t(severity === "info" ? "parameters.severity.info" : "parameters.severity.warning")}</strong>: {describeParameterDiagnosticMessage(t, d)}
            <button onClick={() => copyDetail(d.detail)}>{t("parameters.copyDetails")}</button>
          </li>;
        })}
      </ul>
    </details>
  );
}

export default function ParameterPanel(props: {
  deviceId: number;
  /** Accepted authoritative project snapshot; commands/Undo/Redo invalidate
   * parameters even when the selected device and product language are unchanged. */
  refreshKey?: ProjectTree;
  // Called after every successful `api.setParameterValue`, once per field
  // committed, with the server's own freshly rebuilt `ProjectTree` (fix
  // round 1, item 6) — the same tree `apply(state, cmd)` already built
  // from the genuine post-write `CommandStack` server-side, not a caller-
  // side guess reconstructed from a tree this component never even holds.
  // Required, not optional: every mount site has a `tree` to republish
  // through, and a future mount site that forgot this prop would silently
  // reopen the publish hole this callback exists to close — `tsc` catches
  // that instead.
  onValueApplied: (tree: ProjectTree) => void;
}) {
  const { deviceId, onValueApplied, refreshKey } = props;
  const t = useTranslate();
  const [language] = useProductLanguage();
  const [panel, setPanel] = useState<ParameterPanelDto | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Guards against a slower, earlier device's response landing after a
  // faster, later one's — same stale-reply hazard `CatalogBrowser.tsx`'s
  // `requestIdRef` guards for its own server round trips. A language
  // change re-fetches through this same effect and races the same way a
  // device change always has, so it needs no guard of its own.
  const requestIdRef = useRef(0);

  useEffect(() => {
    setPanel(null);
    setLoadError(null);
    const requestId = ++requestIdRef.current;
    api
      .deviceParameters(deviceId, language)
      .then((p) => {
        if (requestId !== requestIdRef.current) return;
        setPanel(p);
      })
      .catch((e) => {
        if (requestId !== requestIdRef.current) return;
        setLoadError(api.errorMessage(e));
      });
  }, [deviceId, language, refreshKey]);

  if (loadError) {
    return (
      <div className="parameter-panel">
        <h3>{t("parameters.title")}</h3>
        <span className="field-error">{loadError}</span>
      </div>
    );
  }

  if (!panel) {
    return (
      <div className="parameter-panel parameter-panel-loading">
        <h3>{t("parameters.title")}</h3>
        <p className="inspector-description">{t("parameters.loading")}</p>
      </div>
    );
  }

  return (
    <div className="parameter-panel">
      <h3>{t("parameters.title")}</h3>
      {panel.diagnostics.length > 0 && <DiagnosticsBanner diagnostics={panel.diagnostics} />}
      {panel.programId === null ? (
        <p className="inspector-description">{t("parameters.noProgram")}</p>
      ) : (
        panel.sections.map((section, i) => (
          <ParameterSectionView
            key={i}
            section={section}
            diagnostics={panel.diagnostics}
            deviceId={deviceId}
            language={language}
            onUpdated={setPanel}
            onValueApplied={onValueApplied}
          />
        ))
      )}
      {panel.stale.length > 0 && <StaleParametersSection stale={panel.stale} />}
    </div>
  );
}
