/** New-project wizard step: editable areas and lines of the starting topology. */
import type { Translate } from "./i18n";
import {
  DEFAULT_MEDIUM_REF,
  MAX_AREA_OR_LINE,
  newDraftKey,
  nextFreeNumber,
  type DraftArea,
  type StructureIssue,
} from "./projectSeed";
import { IssueList } from "./ProjectWizardIssues";

export default function ProjectWizardTopology(props: {
  areas: DraftArea[];
  issues: readonly StructureIssue[];
  onChange: (areas: DraftArea[]) => void;
  t: Translate;
}) {
  const { areas, issues, onChange, t } = props;
  const setArea = (key: string, update: (area: DraftArea) => DraftArea) =>
    onChange(areas.map((area) => (area.key === key ? update(area) : area)));

  function addArea() {
    const number = nextFreeNumber(areas.map((a) => a.address), MAX_AREA_OR_LINE, 1) ?? 0;
    onChange([...areas, {
      key: newDraftKey(), name: t("projectWizard.topology.defaultArea", { area: number }), address: String(number),
      lines: [],
    }]);
  }

  function addLine(area: DraftArea) {
    const number = nextFreeNumber(area.lines.map((l) => l.address), MAX_AREA_OR_LINE, 1) ?? 0;
    setArea(area.key, (a) => ({
      ...a,
      lines: [...a.lines, {
        key: newDraftKey(), name: t("projectWizard.topology.defaultLine", { area: a.address.trim(), line: number }),
        address: String(number), mediumRef: DEFAULT_MEDIUM_REF,
      }],
    }));
  }

  return (
    <div className="project-wizard-step">
      <p className="project-wizard-intro">{t("projectWizard.topology.intro")}</p>
      {areas.length === 0 && <p className="project-wizard-empty">{t("projectWizard.topology.empty")}</p>}
      <ul className="project-wizard-tree">
        {areas.map((area) => (
          <li key={area.key} className="project-wizard-node">
            <div className="project-wizard-row">
              <input className="project-wizard-number" value={area.address} inputMode="numeric"
                aria-label={t("projectWizard.topology.areaNumber")}
                aria-invalid={issues.some((i) => i.nodeKey === area.key)}
                onChange={(e) => setArea(area.key, (a) => ({ ...a, address: e.target.value }))} />
              <input value={area.name} aria-label={t("projectWizard.topology.areaName")}
                onChange={(e) => setArea(area.key, (a) => ({ ...a, name: e.target.value }))} />
              <button type="button" onClick={() => addLine(area)}>{t("projectWizard.topology.addLine")}</button>
              <button type="button" className="project-wizard-remove" aria-label={t("projectWizard.topology.removeArea", { name: area.name })}
                onClick={() => onChange(areas.filter((a) => a.key !== area.key))}>×</button>
            </div>
            <IssueList issues={issues} nodeKey={area.key} t={t} />
            <ul className="project-wizard-tree">
              {area.lines.map((line) => {
                const setLine = (patch: Partial<typeof line>) => setArea(area.key, (a) => ({
                  ...a, lines: a.lines.map((l) => (l.key === line.key ? { ...l, ...patch } : l)),
                }));
                return (
                  <li key={line.key} className="project-wizard-node">
                    <div className="project-wizard-row">
                      <span className="project-wizard-prefix" aria-hidden="true">{area.address.trim()}.</span>
                      <input className="project-wizard-number" value={line.address} inputMode="numeric"
                        aria-label={t("projectWizard.topology.lineNumber")}
                        aria-invalid={issues.some((i) => i.nodeKey === line.key)}
                        onChange={(e) => setLine({ address: e.target.value })} />
                      <input value={line.name} aria-label={t("projectWizard.topology.lineName")}
                        onChange={(e) => setLine({ name: e.target.value })} />
                      <input className="project-wizard-medium" value={line.mediumRef}
                        aria-label={t("projectWizard.topology.mediumRef")} title={t("projectWizard.topology.mediumHint")}
                        onChange={(e) => setLine({ mediumRef: e.target.value })} />
                      <button type="button" className="project-wizard-remove" aria-label={t("projectWizard.topology.removeLine", { name: line.name })}
                        onClick={() => setArea(area.key, (a) => ({ ...a, lines: a.lines.filter((l) => l.key !== line.key) }))}>×</button>
                    </div>
                    <IssueList issues={issues} nodeKey={line.key} t={t} />
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>
      <button type="button" onClick={addArea}>{t("projectWizard.topology.addArea")}</button>
      <p className="settings-field-hint">{t("projectWizard.topology.mediumHint")}</p>
    </div>
  );
}
