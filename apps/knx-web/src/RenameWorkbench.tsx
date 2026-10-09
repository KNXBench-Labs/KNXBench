/** Shared F2/context-menu rename workflow over explicit project entity targets. */
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import RenameNameField, { RenameScope } from "./RenameNameField";
import { hasRenameContext, renameTarget, type RenameTarget } from "./rename";
import { useTranslate } from "./i18n";

interface Entry { target: RenameTarget; scope: number; server?: string; project?: number }
export default function RenameWorkbench({ tree, scope, onApplied, children }: {
  tree: ProjectTree | null; scope: number; onApplied: (tree: ProjectTree) => void; children: ReactNode;
}) {
  const t = useTranslate();
  const [editor, setEditor] = useState<Entry | null>(null);
  const [pending, setPending] = useState(false);
  const [menu, setMenu] = useState<(Entry & { x: number; y: number }) | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  useEffect(() => { setEditor(null); setMenu(null); setPending(false); },
    [scope, tree?.server_incarnation, tree?.project_incarnation]);
  const current = (entry: Entry | null) => entry && tree && entry.scope === scope
    && entry.server === tree.server_incarnation && entry.project === tree.project_incarnation;
  const visibleMenu = current(menu) ? menu : null;
  useEffect(() => {
    if (!visibleMenu) return;
    const previous = document.activeElement as HTMLElement | null;
    menuRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const outside = (event: PointerEvent) => { if (!menuRef.current?.contains(event.target as Node)) setMenu(null); };
    document.addEventListener("pointerdown", outside);
    return () => { document.removeEventListener("pointerdown", outside); if (previous?.isConnected && !previous.closest("[inert]") && !document.querySelector('[role="dialog"]')) previous.focus(); };
  }, [visibleMenu]);
  function entry(element: EventTarget | null): Entry | null {
    if (!tree || !hasRenameContext(tree) || !(element instanceof Element)
      || element.closest("input, textarea, select, [contenteditable], [role='dialog']")) return null;
    const node = element.closest<HTMLElement>("[data-rename-kind][data-rename-id]");
    const kind = node?.dataset.renameKind; const id = Number(node?.dataset.renameId);
    if ((kind !== "device" && kind !== "group_address") || !Number.isSafeInteger(id)) return null;
    // Canonical device-list catalogue rows also include instances absent from
    // placement branches. Carry their exact displayed name, never a guessed label.
    const target = renameTarget(tree, kind, id) ?? (kind === "device" && node?.dataset.renameValue !== undefined
      ? { kind, id, name: node.dataset.renameValue } : null);
    return target ? { target, scope, server: tree.server_incarnation, project: tree.project_incarnation } : null;
  }
  return <RenameScope.Provider value={scope}><div className="rename-workbench"
    onKeyDownCapture={(event) => {
      if (event.key !== "F2" && event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      const chosen = entry(event.target); if (!chosen) return;
      event.preventDefault(); event.stopPropagation();
      if (event.key === "F2") { setMenu(null); setEditor(chosen); }
      else { const box = (event.target as Element).getBoundingClientRect(); setMenu({ ...chosen, x: box.left, y: box.bottom }); }
    }}
    onContextMenuCapture={(event) => {
      const chosen = entry(event.target); if (!chosen) return;
      event.preventDefault(); setMenu({ ...chosen, x: event.clientX, y: event.clientY });
    }}>
    {children}
    {visibleMenu && <div ref={menuRef} role="menu" className="rename-context-menu"
      style={{ left: Math.max(8, Math.min(visibleMenu.x, window.innerWidth - 200)),
        top: Math.max(8, Math.min(visibleMenu.y, window.innerHeight - 72)) }}
      onKeyDown={(event) => { if (event.key === "Escape" || event.key === "Tab") { event.preventDefault(); event.stopPropagation(); setMenu(null); } }}>
      <button type="button" role="menuitem" onClick={() => { setEditor(visibleMenu); setMenu(null); }}>{t("rename.title")} <kbd>F2</kbd></button>
    </div>}
    {current(editor) && editor && tree && <Overlay label={t("rename.title")} className="rename-dialog" onClose={() => { if (!pending) setEditor(null); }}>
      <h2>{t("rename.title")}</h2>
      <RenameNameField tree={tree} target={editor.target} onApplied={onApplied} autoFocus onPendingChange={setPending} onDone={() => { setPending(false); setEditor(null); }} />
    </Overlay>}
  </div></RenameScope.Provider>;
}
