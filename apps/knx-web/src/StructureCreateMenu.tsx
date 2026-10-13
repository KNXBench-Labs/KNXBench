/** One `+ New` menu per structure page; each entry opens the existing form (#12, #15). */
import { useRef, useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { NewAreaRow, NewBuildingPartRow, NewGroupAddressRow, NewGroupRangeRow, NewLineRow } from "./ProjectExplorer";
import { findCreateEntry, type CreateEntry, type CreateGroup } from "./structureCreateEntries";

type Translate = ReturnType<typeof useTranslate>;

export function createEntryLabel(t: Translate, entry: CreateEntry): string {
  switch (entry.kind) {
    case "device": return t("structure.addDevice");
    case "area": return t("structure.addArea");
    case "line": return t("structure.addLineIn", { name: entry.parentName ?? "" });
    case "site-root": return t("structure.addSite");
    case "building-root": return t("structure.addRootBuilding");
    case "building-child": return t("structure.addBuildingIn", { name: entry.parentName ?? "" });
    case "range-root": return t("structure.addRootRange");
    case "range-child": return t("structure.addRangeIn", { name: entry.parentName ?? "" });
    case "group-address": return entry.rangeId != null
      ? t("structure.addGroupAddressIn", { name: entry.parentName ?? "" }) : t("structure.addGroupAddress");
  }
}

function CreateForm(props: { tree: ProjectTree; entry: CreateEntry; onCreated: (tree: ProjectTree) => void }) {
  const { entry, onCreated } = props;
  const installationId = entry.installationId ?? undefined;
  switch (entry.kind) {
    case "area": return <NewAreaRow installationId={installationId} onCreated={onCreated} />;
    case "line": return <NewLineRow areaId={entry.parentId!} onCreated={onCreated} />;
    case "site-root": return <NewBuildingPartRow fixedKind="Ground" installationId={installationId} onCreated={onCreated} />;
    case "building-root": return <NewBuildingPartRow installationId={installationId} onCreated={onCreated} />;
    case "building-child": return <NewBuildingPartRow parentId={entry.parentId} onCreated={onCreated} />;
    case "range-root": return <NewGroupRangeRow installationId={installationId} onCreated={onCreated} />;
    case "range-child": return <NewGroupRangeRow parentId={entry.parentId} onCreated={onCreated} />;
    case "group-address": {
      const installation = props.tree.installations.find(candidate => candidate.id === entry.installationId);
      return <NewGroupAddressRow ranges={installation?.group_ranges ?? []} installationId={entry.installationId!}
        initialRangeId={entry.rangeId ?? null} onCreated={onCreated} />;
    }
    case "device": return null;
  }
}

/** The header button and its menu. */
export function StructureCreateMenu(props: { groups: CreateGroup[]; onChoose: (entry: CreateEntry) => void }) {
  const t = useTranslate();
  const menu = useRef<HTMLDetailsElement>(null);
  const grouped = props.groups.filter(group => group.installationId !== null).length > 1;
  const close = () => { if (menu.current) menu.current.open = false; };
  // Opens towards the button's end unless that would cross into the explorer; then towards its start.
  const [align, setAlign] = useState<"start" | "end">("end");
  const onToggle = () => {
    const details = menu.current;
    if (!details?.open) return;
    const page = (details.closest(".workbench-center") ?? document.documentElement).getBoundingClientRect();
    const button = details.querySelector("summary")!.getBoundingClientRect();
    const width = details.querySelector<HTMLElement>(".structure-new-menu-content")?.offsetWidth ?? 0;
    setAlign(button.right - width >= page.left ? "end" : "start");
  };
  return <details ref={menu} className="structure-new-menu" onToggle={onToggle} onKeyDown={(event) => {
    if (event.key === "Escape") { event.stopPropagation(); close(); menu.current?.querySelector("summary")?.focus(); }
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>("[data-create-entry]")];
    if (items.length === 0) return;
    event.preventDefault();
    const position = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "ArrowDown" ? (position + 1) % items.length : (position <= 0 ? items.length - 1 : position - 1);
    items[next].focus();
  }}>
    <summary>+ {t("structure.new")}</summary>
    <div className="structure-new-menu-content" data-align={align} role="group" aria-label={t("structure.newMenu")}>
      {props.groups.map(group => <div key={group.installationId ?? "project"} className="structure-new-group"
        data-installation-id={group.installationId ?? undefined}>
        {grouped && group.installationName !== null && <p className="structure-new-group-heading">{group.installationName}</p>}
        {group.entries.map(entry => <button key={entry.key} type="button" data-create-entry={entry.kind}
          data-installation-id={entry.installationId ?? undefined} onClick={() => { close(); props.onChoose(entry); }}>
          {createEntryLabel(t, entry)}</button>)}
      </div>)}
    </div>
  </details>;
}

/** The chosen form, below the header; it stays open for several creations until closed. */
export function StructureCreatePanel(props: { tree: ProjectTree; groups: CreateGroup[]; activeKey: string | null;
  onClose: () => void; onCreated: (tree: ProjectTree) => void }) {
  const t = useTranslate();
  const entry = findCreateEntry(props.groups, props.activeKey);
  if (entry === null || entry.kind === "device") return null;
  const label = createEntryLabel(t, entry);
  const installation = props.tree.installations.find(candidate => candidate.id === entry.installationId);
  return <section className="structure-create" data-structure-create={entry.kind} data-parent-id={entry.parentId}
    data-installation-id={entry.installationId ?? undefined} aria-label={label}
    onKeyDown={(event) => { if (event.key === "Escape") { event.stopPropagation(); props.onClose(); } }}>
    <header className="structure-create-heading">
      <h2>{label}</h2>
      {props.tree.installations.length > 1 && installation && <p className="structure-create-target">{installation.name}</p>}
      <button type="button" className="structure-create-close" onClick={props.onClose}>{t("structure.closeCreate")}</button>
    </header>
    {entry.kind === "site-root" && <p className="structure-create-hint">{t("structure.siteHint")}</p>}
    <ul key={entry.key}><CreateForm tree={props.tree} entry={entry} onCreated={props.onCreated} /></ul>
  </section>;
}

/** Menu state for one page: which entry's form is open. */
export function useStructureCreate() {
  const [activeKey, setActiveKey] = useState<string | null>(null);
  return { activeKey, open: (entry: CreateEntry) => setActiveKey(entry.key), close: () => setActiveKey(null) };
}
