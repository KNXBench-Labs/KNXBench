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

// T18 slice 3, task 4 (design docs/superpowers/specs/2026-09-11-parameter-editor-design.md).
// Fetches `GET /api/device/{id}/parameters` on every device selection —
// unconditionally, not gated on a `program_id` the client cannot see
// (`DeviceDetail` carries no such field): a device whose program does not
// resolve still returns 200 with `programId: null`, an empty `sections`,
// and — critically — that device's own `stale` entries (D21), which a
// conditional fetch would have hidden exactly where they matter most.

// D23's own fallback: `module_id` when present, else `"Module #{module_node}"`.
function sectionLabel(scope: ModuleScope | null): string {
  if (scope === null) return "Device";
  return scope.moduleId ?? `Module #${scope.moduleNode}`;
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
  onUpdated: (panel: ParameterPanelDto) => void;
}) {
  const { field, deviceId, onUpdated } = props;
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
      const panel = await api.setParameterValue(deviceId, field.etsId, value);
      onUpdated(panel);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  const label = field.name ?? field.text ?? field.etsId;

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
          <option value="">(none)</option>
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
          Shared across every instantiation of this module; read-only in this release.
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
  onUpdated: (panel: ParameterPanelDto) => void;
}) {
  const { section, deviceId, onUpdated } = props;
  return (
    <details className="parameter-section" open>
      <summary>{sectionLabel(section.scope)}</summary>
      <div className="parameter-fields">
        {section.fields.map((field) => (
          <ParameterFieldRow
            key={field.etsId}
            field={field}
            deviceId={deviceId}
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
  return (
    <details className="parameter-stale-section" open>
      <summary>Stale values ({stale.length})</summary>
      <p className="inspector-description">
        These stored values no longer correspond to any parameter in the current application
        program.
      </p>
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

  function copyDetail(detail: string) {
    void navigator.clipboard?.writeText(detail);
  }

  return (
    <details className="parameter-diagnostics-banner">
      <summary>
        {diagnostics.length} issue{diagnostics.length === 1 ? "" : "s"} found while evaluating
        this device&apos;s parameters
      </summary>
      <ul>
        {diagnostics.map((d, i) => (
          <li key={i}>
            {d.message}
            <button onClick={() => copyDetail(d.detail)}>Copy details</button>
          </li>
        ))}
      </ul>
    </details>
  );
}

export default function ParameterPanel(props: { deviceId: number }) {
  const { deviceId } = props;
  const [panel, setPanel] = useState<ParameterPanelDto | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Guards against a slower, earlier device's response landing after a
  // faster, later one's — same stale-reply hazard `CatalogBrowser.tsx`'s
  // `requestIdRef` guards for its own server round trips.
  const requestIdRef = useRef(0);

  useEffect(() => {
    setPanel(null);
    setLoadError(null);
    const requestId = ++requestIdRef.current;
    api
      .deviceParameters(deviceId)
      .then((p) => {
        if (requestId !== requestIdRef.current) return;
        setPanel(p);
      })
      .catch((e) => {
        if (requestId !== requestIdRef.current) return;
        setLoadError(api.errorMessage(e));
      });
  }, [deviceId]);

  if (loadError) {
    return (
      <div className="parameter-panel">
        <h3>Parameters</h3>
        <span className="field-error">{loadError}</span>
      </div>
    );
  }

  if (!panel) {
    return (
      <div className="parameter-panel parameter-panel-loading">
        <h3>Parameters</h3>
        <p className="inspector-description">Loading parameters…</p>
      </div>
    );
  }

  return (
    <div className="parameter-panel">
      <h3>Parameters</h3>
      {panel.diagnostics.length > 0 && <DiagnosticsBanner diagnostics={panel.diagnostics} />}
      {panel.programId === null ? (
        <p className="inspector-description">
          This device has no resolvable application program; parameters cannot be shown.
        </p>
      ) : (
        panel.sections.map((section, i) => (
          <ParameterSectionView
            key={i}
            section={section}
            deviceId={deviceId}
            onUpdated={setPanel}
          />
        ))
      )}
      {panel.stale.length > 0 && <StaleParametersSection stale={panel.stale} />}
    </div>
  );
}
