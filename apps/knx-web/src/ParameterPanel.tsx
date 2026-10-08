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
import { LanguageFallbackBadge } from "./languageFallback";
import { groupParameterDiagnostics, isManufacturerRestricted } from "./parameterPresentation";
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
/**
 * AR10: which part of a field's visible text fell back to the program's own
 * text. Only the label actually shown counts (`text` before `name`, see
 * `ParameterFieldRow`); an option without a label shows its value, which no
 * translation could change.
 */
export function untranslatedPart(field: ParameterField): "label" | "options" | null {
  const shownLanguage = field.text !== null ? field.textLanguage : field.name !== null ? field.nameLanguage : undefined;
  if (shownLanguage === null) return "label";
  if (field.enumOptions.some((option) => option.text !== null && option.language === null)) return "options";
  return null;
}

function ParameterFieldRow(props: {
  field: ParameterField;
  /** In a module instantiation's section, a disabled field is shared by
   * every instantiation; at device level it is refused for another reason
   * (ADR-0080: access, manufacturer calculation, unrecorded authority). */
  moduleScoped: boolean;
  deviceId: number;
  language: string | null;
  sourceLanguage: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
  onValueApplied: (tree: ProjectTree) => void;
}) {
  const { field, moduleScoped, deviceId, language, sourceLanguage, onUpdated, onValueApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(field.value ?? "");
  const [error, setError] = useState<string | null>(null);
  // Belt-and-braces on purpose (see `api.ts`'s `writeEtsId` doc comment):
  // the server's contract makes the two conditions exact opposites of
  // each other, but the control checks both rather than trusting either
  // one alone.
  const disabled = isManufacturerRestricted(field) || !field.editable || field.writeEtsId === null;

  useEffect(() => {
    setValue(field.value ?? "");
    setError(null);
  }, [field.etsId, field.value]);

  async function apply() {
    if (isManufacturerRestricted(field) || !field.editable || field.writeEtsId === null) return;
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
  // No product language selected means the package's own text is exactly
  // what was asked for, so nothing fell back.
  const untranslated = language === null ? null : untranslatedPart(field);

  return (
    <label className="inspector-field parameter-field" data-ets-id={field.etsId}>
      {label}
      {field.access && <span className="provenance-badge">{field.access}</span>}
      {untranslated && language !== null && (
        <LanguageFallbackBadge selected={language} source={sourceLanguage} part={untranslated} />
      )}
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
          {t(field.access === "None" ? "parameters.accessNoneCaption"
            : field.access === "Read" ? "parameters.accessReadCaption"
            : moduleScoped ? "parameters.sharedReadOnlyCaption" : "parameters.readOnlyCaption")}
        </span>
      )}
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

// AR10: one line naming how many fields show the program's own text
// instead of the selected product language, including folded ones.
function UntranslatedSummary(props: { panel: ParameterPanelDto; language: string | null }) {
  const { panel, language } = props;
  const t = useTranslate();
  if (language === null) return null;
  const count = panel.sections.reduce(
    (sum, section) => sum + section.fields.filter((field) => untranslatedPart(field) !== null).length, 0);
  if (count === 0) return null;
  const plural = count === 1 ? "one" : "other";
  return (
    <p className="inspector-description parameter-language-summary">
      {panel.sourceLanguage === null
        ? t(`parameters.untranslated.summaryUnknown.${plural}` as const, { count, language })
        : t(`parameters.untranslated.summary.${plural}` as const, { count, language, source: panel.sourceLanguage })}
    </p>
  );
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
  unsupportedControlKind: "parameters.diagnostic.unsupportedControlKind",
  evaluationWorkBudgetExhausted: "parameters.diagnostic.evaluationWorkBudgetExhausted",
  parameterAccessReadOnly: "parameters.diagnostic.parameterAccessReadOnly",
  manufacturerCalculation: "parameters.diagnostic.manufacturerCalculation",
  writeAuthorityUnavailable: "parameters.diagnostic.writeAuthorityUnavailable",
};

function describeParameterDiagnosticMessage(t: Translate, diagnostic: ParameterDiagnostic): string {
  const key = PARAMETER_DIAGNOSTIC_MESSAGE_KEYS[diagnostic.kind];
  return key ? t(key) : diagnostic.message;
}

// One collapsible group per device/module scope. Diagnostic prose belongs
// only in the Diagnostics tab; field-level refusal captions remain local.
function ParameterSectionView(props: {
  section: ParameterSection;
  deviceId: number;
  language: string | null;
  sourceLanguage: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
  onValueApplied: (tree: ProjectTree) => void;
}) {
  const { section, deviceId, language, sourceLanguage, onUpdated, onValueApplied } = props;
  const t = useTranslate();
  const fields = section.fields;
  return (
    <details className="parameter-section" open>
      <summary>{sectionLabel(t, section.scope)}</summary>
      <div className="parameter-fields">
        {fields.map((field) => (
          <ParameterFieldRow
            key={field.etsId}
            field={field}
            moduleScoped={section.scope !== null}
            deviceId={deviceId}
            language={language}
            sourceLanguage={sourceLanguage}
            onUpdated={onUpdated}
            onValueApplied={onValueApplied}
          />
        ))}
      </div>
    </details>
  );
}

// D21: stored-but-unmatched values remain inspectable in Diagnostics.
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

// Count source occurrences, group repeated headlines without losing details.
// Technical records stay behind an explicit disclosure; unknown severities
// remain warnings, never silent informational labels.
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
    <details className="parameter-diagnostics-banner" data-has-warnings={warnings > 0} open>
      <summary>{summary}</summary>
      <ul>
        {groupParameterDiagnostics(diagnostics).map((group, i) => {
          const d = group[0];
          const severity = d.severity === "info" ? "info" : "warning";
          return <li key={i} data-severity={severity}>
            <strong>{t(severity === "info" ? "parameters.severity.info" : "parameters.severity.warning")}</strong>: {describeParameterDiagnosticMessage(t, d)}
            <span className="parameter-diagnostic-scope">{sectionLabel(t, d.scope)}</span>
            {group.length > 1 && <span className="provenance-badge">{t("parameters.occurrences", { count: group.length })}</span>}
            <button type="button" onClick={() => copyDetail(group.map((item) => item.detail).join("\n"))}>{t("parameters.copyDetails")}</button>
            <details className="parameter-diagnostic-details">
              <summary>{t("parameters.technicalDetails")}</summary>
              {group.map((item, index) => <pre key={index}>{item.detail}</pre>)}
            </details>
          </li>;
        })}
      </ul>
    </details>
  );
}

export type ParameterView = "parameters" | "diagnostics" | "restricted";

type ParameterPanelProps = {
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
  view?: ParameterView;
};

/** One read model shared by the editor and both inspection-only tabs. */
export function useDeviceParameters(deviceId: number, refreshKey?: ProjectTree) {
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

  return { panel, loadError, language, setPanel };
}

export function ParameterPanelContent(props: ParameterPanelProps & {
  state: ReturnType<typeof useDeviceParameters>;
}) {
  const { deviceId, onValueApplied, view = "parameters", state } = props;
  const { panel, loadError, language, setPanel } = state;
  const t = useTranslate();
  const title = view === "diagnostics" ? "parameters.diagnosticsTab"
    : view === "restricted" ? "parameters.restrictedTab" : "parameters.title";

  if (loadError) {
    return (
      <div className="parameter-panel">
        <h3>{t(title)}</h3>
        <span className="field-error">{loadError}</span>
      </div>
    );
  }

  if (!panel) {
    return (
      <div className="parameter-panel parameter-panel-loading">
        <h3>{t(title)}</h3>
        <p className="inspector-description">{t("parameters.loading")}</p>
      </div>
    );
  }

  if (view === "diagnostics") {
    return <div className="parameter-panel parameter-diagnostics">
      <h3>{t(title)}</h3>
      {panel.diagnostics.length > 0 ? <DiagnosticsBanner diagnostics={panel.diagnostics} />
        : <p className="inspector-description">{t("parameters.noDiagnostics")}</p>}
      <UntranslatedSummary panel={panel} language={language} />
      {panel.stale.length > 0 && <StaleParametersSection stale={panel.stale} />}
    </div>;
  }

  const sections = panel.sections.map((section) => ({
    ...section,
    fields: section.fields.filter((field) => isManufacturerRestricted(field) === (view === "restricted")),
  })).filter((section) => section.fields.length > 0);

  return (
    <div className="parameter-panel">
      <h3>{t(title)}</h3>
      {view === "restricted" && <p className="inspector-description">{t("parameters.restrictedDescription")}</p>}
      {panel.programId === null ? (
        <p className="inspector-description">{t("parameters.noProgram")}</p>
      ) : sections.length === 0 ? (
        <p className="inspector-description">{t(view === "restricted" ? "parameters.noRestrictedFields" : "parameters.noUserFields")}</p>
      ) : sections.map((section, i) => (
        <ParameterSectionView
          key={i}
          section={section}
          deviceId={deviceId}
          language={language}
          sourceLanguage={panel.sourceLanguage}
          onUpdated={setPanel}
          onValueApplied={onValueApplied}
        />
      ))}
    </div>
  );
}

export default function ParameterPanel(props: ParameterPanelProps) {
  const state = useDeviceParameters(props.deviceId, props.refreshKey);
  return <ParameterPanelContent {...props} state={state} />;
}
