/** New-project wizard step: main and middle group ranges, optionally pre-filled from a preset. */
import { useState } from "react";
import type { GroupAddressStyle } from "./api";
import type { Translate } from "./i18n";
import { BUNDLED_GROUP_PRESETS, expandPreset, type PresetId } from "./groupStructurePresets";
import {
  MAX_MAIN_GROUP,
  MAX_MIDDLE_GROUP,
  newDraftKey,
  nextFreeNumber,
  type DraftBuildingPart,
  type DraftMainRange,
  type StructureIssue,
} from "./projectSeed";
import { IssueList } from "./ProjectWizardIssues";

export default function ProjectWizardGroups(props: {
  groupRanges: DraftMainRange[];
  buildings: readonly DraftBuildingPart[];
  style: GroupAddressStyle;
  issues: readonly StructureIssue[];
  onChange: (groupRanges: DraftMainRange[]) => void;
  t: Translate;
}) {
  const { groupRanges, buildings, style, issues, onChange, t } = props;
  const [presetId, setPresetId] = useState<PresetId>(BUNDLED_GROUP_PRESETS[0].id);
  const [presetError, setPresetError] = useState<string | null>(null);
  const threeLevel = style === "ThreeLevel";
  const setMain = (key: string, update: (main: DraftMainRange) => DraftMainRange) =>
    onChange(groupRanges.map((main) => (main.key === key ? update(main) : main)));

  function applyPreset() {
    const preset = BUNDLED_GROUP_PRESETS.find((p) => p.id === presetId);
    if (!preset) return;
    const result = expandPreset(preset, buildings, style, (key) => t(key));
    if (!result.ok) {
      setPresetError(t(result.message, result.params));
      return;
    }
    setPresetError(null);
    onChange(result.groupRanges);
  }

  function addMain() {
    const number = nextFreeNumber(groupRanges.map((m) => m.main), MAX_MAIN_GROUP, 1) ?? 0;
    onChange([...groupRanges, {
      key: newDraftKey(), name: t("projectWizard.groups.defaultMain", { n: number }), main: String(number), middles: [],
    }]);
  }

  function addMiddle(main: DraftMainRange) {
    const number = nextFreeNumber(main.middles.map((m) => m.middle), MAX_MIDDLE_GROUP) ?? 0;
    setMain(main.key, (m) => ({
      ...m,
      middles: [...m.middles, { key: newDraftKey(), name: t("projectWizard.groups.defaultMiddle", { n: number }), middle: String(number) }],
    }));
  }

  if (style === "Free") {
    return (
      <div className="project-wizard-step">
        <p className="project-wizard-intro">{t("projectWizard.groups.freeHint")}</p>
        <IssueList issues={issues} nodeKey={null} t={t} />
      </div>
    );
  }

  return (
    <div className="project-wizard-step">
      <p className="project-wizard-intro">{t("projectWizard.groups.intro")}</p>
      {!threeLevel && <p className="settings-field-hint">{t("projectWizard.groups.twoLevelHint")}</p>}
      <div className="project-wizard-row project-wizard-preset">
        <label>
          <span>{t("projectWizard.groups.preset")}</span>
          <select value={presetId} onChange={(e) => { setPresetId(e.target.value as PresetId); setPresetError(null); }}>
            {BUNDLED_GROUP_PRESETS.map((preset) => (
              <option key={preset.id} value={preset.id}>{t(preset.label)}</option>
            ))}
          </select>
        </label>
        <button type="button" onClick={applyPreset}>{t("projectWizard.groups.applyPreset")}</button>
      </div>
      {presetError && <p className="field-error" role="alert">{presetError}</p>}
      {groupRanges.length === 0 && <p className="project-wizard-empty">{t("projectWizard.groups.empty")}</p>}
      <ul className="project-wizard-tree">
        {groupRanges.map((main) => (
          <li key={main.key} className="project-wizard-node">
            <div className="project-wizard-row">
              <input className="project-wizard-number" value={main.main} inputMode="numeric"
                aria-label={t("projectWizard.groups.mainNumber")}
                aria-invalid={issues.some((i) => i.nodeKey === main.key)}
                onChange={(e) => setMain(main.key, (m) => ({ ...m, main: e.target.value }))} />
              <input value={main.name} aria-label={t("projectWizard.groups.mainName")}
                onChange={(e) => setMain(main.key, (m) => ({ ...m, name: e.target.value }))} />
              {threeLevel && (
                <button type="button" onClick={() => addMiddle(main)}>{t("projectWizard.groups.addMiddle")}</button>
              )}
              <button type="button" className="project-wizard-remove" aria-label={t("projectWizard.groups.removeMain", { name: main.name })}
                onClick={() => onChange(groupRanges.filter((m) => m.key !== main.key))}>×</button>
            </div>
            <IssueList issues={issues} nodeKey={main.key} t={t} />
            <ul className="project-wizard-tree">
              {main.middles.map((middle) => {
                const setMiddle = (patch: Partial<typeof middle>) => setMain(main.key, (m) => ({
                  ...m, middles: m.middles.map((x) => (x.key === middle.key ? { ...x, ...patch } : x)),
                }));
                return (
                  <li key={middle.key} className="project-wizard-node">
                    <div className="project-wizard-row">
                      <span className="project-wizard-prefix" aria-hidden="true">{main.main.trim()}/</span>
                      <input className="project-wizard-number" value={middle.middle} inputMode="numeric"
                        aria-label={t("projectWizard.groups.middleNumber")}
                        aria-invalid={issues.some((i) => i.nodeKey === middle.key)}
                        onChange={(e) => setMiddle({ middle: e.target.value })} />
                      <input value={middle.name} aria-label={t("projectWizard.groups.middleName")}
                        onChange={(e) => setMiddle({ name: e.target.value })} />
                      <button type="button" className="project-wizard-remove" aria-label={t("projectWizard.groups.removeMiddle", { name: middle.name })}
                        onClick={() => setMain(main.key, (m) => ({ ...m, middles: m.middles.filter((x) => x.key !== middle.key) }))}>×</button>
                    </div>
                    <IssueList issues={issues} nodeKey={middle.key} t={t} />
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>
      <button type="button" onClick={addMain}>{t("projectWizard.groups.addMain")}</button>
      <IssueList issues={issues} nodeKey={null} t={t} />
    </div>
  );
}
