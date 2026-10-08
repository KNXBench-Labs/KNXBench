/** New-project wizard step: a small tree editor for buildings, floors, rooms and boards. */
import { useState } from "react";
import type { Translate, MessageKey } from "./i18n";
import {
  newDraftKey,
  updateBuildingPart,
  type DraftBuildingPart,
  type SeedBuildingKind,
  type StructureIssue,
} from "./projectSeed";
import { IssueList } from "./ProjectWizardIssues";

/** Which kinds may be added under which; rooms and boards are leaves here. */
const CHILD_KINDS: Record<SeedBuildingKind, readonly SeedBuildingKind[]> = {
  Building: ["Floor", "DistributionBoard"],
  Floor: ["Room", "DistributionBoard"],
  Room: [],
  DistributionBoard: [],
};

const KIND_LABEL: Record<SeedBuildingKind, MessageKey> = {
  Building: "projectWizard.building.kind.Building",
  Floor: "projectWizard.building.kind.Floor",
  Room: "projectWizard.building.kind.Room",
  DistributionBoard: "projectWizard.building.kind.DistributionBoard",
};

const ADD_LABEL: Record<SeedBuildingKind, MessageKey> = {
  Building: "projectWizard.building.addBuilding",
  Floor: "projectWizard.building.addFloor",
  Room: "projectWizard.building.addRoom",
  DistributionBoard: "projectWizard.building.addDistributionBoard",
};

const DEFAULT_NAME: Record<SeedBuildingKind, MessageKey> = {
  Building: "projectWizard.building.defaultBuilding",
  Floor: "projectWizard.building.defaultFloor",
  Room: "projectWizard.building.defaultRoom",
  DistributionBoard: "projectWizard.building.defaultDistributionBoard",
};

/** Upper bound of the quick floor fill; larger trees are built by hand or later. */
const MAX_QUICK_FLOORS = 50;

function newPart(t: Translate, kind: SeedBuildingKind, n: number): DraftBuildingPart {
  return { key: newDraftKey(), name: t(DEFAULT_NAME[kind], { n }), kind, children: [] };
}

export default function ProjectWizardBuilding(props: {
  buildings: DraftBuildingPart[];
  issues: readonly StructureIssue[];
  onChange: (buildings: DraftBuildingPart[]) => void;
  t: Translate;
}) {
  const { buildings, issues, onChange, t } = props;

  function addChild(parent: DraftBuildingPart, kind: SeedBuildingKind) {
    const siblings = parent.children.filter((c) => c.kind === kind).length;
    onChange(updateBuildingPart(buildings, parent.key, (p) => ({
      ...p, children: [...p.children, newPart(t, kind, siblings + 1)],
    })));
  }

  return (
    <div className="project-wizard-step">
      <p className="project-wizard-intro">{t("projectWizard.building.intro")}</p>
      {buildings.length === 0 && <p className="project-wizard-empty">{t("projectWizard.building.empty")}</p>}
      <ul className="project-wizard-tree">
        {buildings.map((part) => (
          <BuildingNode key={part.key} part={part} issues={issues} t={t} root
            onRename={(key, name) => onChange(updateBuildingPart(buildings, key, (p) => ({ ...p, name })))}
            onRemove={(key) => onChange(updateBuildingPart(buildings, key, () => null))}
            onAddChild={addChild}
            onAddFloors={(building, count) => {
              const existing = building.children.filter((c) => c.kind === "Floor").length;
              const floors = Array.from({ length: count }, (_, i) => newPart(t, "Floor", existing + i + 1));
              onChange(updateBuildingPart(buildings, building.key, (p) => ({ ...p, children: [...p.children, ...floors] })));
            }} />
        ))}
      </ul>
      <button type="button" onClick={() => onChange([...buildings, newPart(t, "Building", buildings.length + 1)])}>
        {t("projectWizard.building.addBuilding")}
      </button>
      <IssueList issues={issues} nodeKey={null} t={t} />
    </div>
  );
}

function BuildingNode(props: {
  part: DraftBuildingPart;
  issues: readonly StructureIssue[];
  t: Translate;
  root?: boolean;
  onRename: (key: string, name: string) => void;
  onRemove: (key: string) => void;
  onAddChild: (parent: DraftBuildingPart, kind: SeedBuildingKind) => void;
  onAddFloors: (building: DraftBuildingPart, count: number) => void;
}) {
  const { part, issues, t, root = false, onRename, onRemove, onAddChild, onAddFloors } = props;
  const [floorCount, setFloorCount] = useState("3");
  const floorNumber = Number(floorCount);
  const floorCountValid = /^\d+$/.test(floorCount.trim()) && floorNumber >= 1 && floorNumber <= MAX_QUICK_FLOORS;
  return (
    <li className="project-wizard-node">
      <div className="project-wizard-row">
        <span className="project-wizard-kind">{t(KIND_LABEL[part.kind])}</span>
        <input value={part.name} aria-label={t("projectWizard.building.partName", { kind: t(KIND_LABEL[part.kind]) })}
          aria-invalid={issues.some((i) => i.nodeKey === part.key)}
          onChange={(e) => onRename(part.key, e.target.value)} />
        {CHILD_KINDS[part.kind].map((kind) => (
          <button key={kind} type="button" onClick={() => onAddChild(part, kind)}>
            {t(ADD_LABEL[kind])}
          </button>
        ))}
        <button type="button" className="project-wizard-remove" aria-label={t("projectWizard.building.remove", { name: part.name })}
          onClick={() => onRemove(part.key)}>×</button>
      </div>
      <IssueList issues={issues} nodeKey={part.key} t={t} />
      {root && part.kind === "Building" && (
        <div className="project-wizard-row project-wizard-quick">
          <label>
            <span>{t("projectWizard.building.quickFloors")}</span>
            <input className="project-wizard-number" value={floorCount} inputMode="numeric"
              aria-invalid={!floorCountValid} onChange={(e) => setFloorCount(e.target.value)} />
          </label>
          <button type="button" disabled={!floorCountValid} onClick={() => onAddFloors(part, floorNumber)}>
            {t("projectWizard.building.quickFloorsButton")}
          </button>
        </div>
      )}
      {part.children.length > 0 && (
        <ul className="project-wizard-tree">
          {part.children.map((child) => (
            <BuildingNode key={child.key} part={child} issues={issues} t={t}
              onRename={onRename} onRemove={onRemove} onAddChild={onAddChild} onAddFloors={onAddFloors} />
          ))}
        </ul>
      )}
    </li>
  );
}
