import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type {
  ModuleScope,
  ParameterDiagnostic,
  ParameterField,
  ParameterPanel as ParameterPanelDto,
  ParameterSection,
  StaleParameter,
} from "./api";
import { useProductLanguage } from "./productLanguage";
import { useTranslate, type Translate } from "./i18n";

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
// commands. A module-scoped field (`editable: false`, D25) renders its
// input disabled, never hidden, with a caption naming the read-only
// reason instead of leaving the user to guess at a silently inert field.
function ParameterFieldRow(props: {
  field: ParameterField;
  deviceId: number;
  language: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
}) {
  const { field, deviceId, language, onUpdated } = props;
  const t = useTranslate();
  const [value, setValue] = useState(field.value ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(field.value ?? "");
    setError(null);
  }, [field.etsId, field.value]);

  async function apply() {
    if (!field.editable) return;
    const current = field.value ?? "";
    if (value === current) return;
    setError(null);
    try {
      const panel = await api.setParameterValue(deviceId, field.etsId, value, language);
      onUpdated(panel);
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
          disabled={!field.editable}
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
      ) : field.kind === "Number" ? (
        <input
          type="number"
          value={value}
          min={field.min ?? undefined}
          max={field.max ?? undefined}
          disabled={!field.editable}
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
          disabled={!field.editable}
          onChange={(e) => setValue(e.target.value)}
          onBlur={apply}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      )}
      {!field.editable && (
        <span className="parameter-field-caption">
          {t("parameters.sharedReadOnlyCaption")}
        </span>
      )}
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

// One collapsible group per `ParameterSectionDto` — the top-level
// (`scope: null`) section and one per module instantiation (D23: "12
// instantiations are 12 results," never collapsed back into one list).
function ParameterSectionView(props: {
  section: ParameterSection;
  deviceId: number;
  language: string | null;
  onUpdated: (panel: ParameterPanelDto) => void;
}) {
  const { section, deviceId, language, onUpdated } = props;
  const t = useTranslate();
  return (
    <details className="parameter-section" open>
      <summary>{sectionLabel(t, section.scope)}</summary>
      <div className="parameter-fields">
        {section.fields.map((field) => (
          <ParameterFieldRow
            key={field.etsId}
            field={field}
            deviceId={deviceId}
            language={language}
            onUpdated={onUpdated}
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
// behind a "copy details" affordance, not printed inline.
function DiagnosticsBanner(props: { diagnostics: ParameterDiagnostic[] }) {
  const { diagnostics } = props;
  const t = useTranslate();

  function copyDetail(detail: string) {
    void navigator.clipboard?.writeText(detail);
  }

  return (
    <details className="parameter-diagnostics-banner">
      <summary>{t("parameters.diagnosticsCount", { count: diagnostics.length })}</summary>
      <ul>
        {diagnostics.map((d, i) => (
          <li key={i}>
            {d.message}
            <button onClick={() => copyDetail(d.detail)}>{t("parameters.copyDetails")}</button>
          </li>
        ))}
      </ul>
    </details>
  );
}

export default function ParameterPanel(props: { deviceId: number }) {
  const { deviceId } = props;
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
  }, [deviceId, language]);

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
            deviceId={deviceId}
            language={language}
            onUpdated={setPanel}
          />
        ))
      )}
      {panel.stale.length > 0 && <StaleParametersSection stale={panel.stale} />}
    </div>
  );
}
