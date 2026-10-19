/** Explicit selection, read-only preview and backed-up catalogue placement cleanup. */
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogCleanupPreview, CatalogItem } from "./api";
import { useTranslate } from "./i18n";
import { useProductLanguage } from "./productLanguage";
import Overlay from "./Overlay";
import CatalogItemIdentity from "./CatalogItemIdentity";
import { redundantCatalogPlacements } from "./catalogCleanup";

export interface CatalogCleanupResult { removedItemIds: string[]; backup: string }

/** The values a row shows, lower-cased: the filter matches these, never field names. */
function searchText(item: CatalogItem): string {
  const d = item.details;
  return [item.name, item.number, item.id, item.productRefId, item.hardware2programRefId, d?.applicationProgramId,
    d?.applicationName, d?.applicationVersion, d?.catalogSectionId, ...(d?.sourceNames ?? [])]
    .filter((value): value is string => typeof value === "string").join("\n").toLowerCase();
}

export default function CatalogCleanupDialog(props: {
  /** Called once with the confirmed result; the owner keeps it after the dialog closes. */
  onRemoved: (result: CatalogCleanupResult) => void | Promise<void>;
  onClose: () => void;
}) {
  const t = useTranslate();
  const [language] = useProductLanguage();
  const [items, setItems] = useState<CatalogItem[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [preview, setPreview] = useState<CatalogCleanupPreview | null>(null);
  const [done, setDone] = useState<CatalogCleanupResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [blocked, setBlocked] = useState(false);
  const [working, setWorking] = useState(false);
  const pending = useRef(false);
  const alive = useRef(true);
  const closeRef = useRef<HTMLButtonElement>(null);
  const statusRef = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    alive.current = true;
    return () => { alive.current = false; };
  }, []);

  async function load() {
    if (pending.current) return;
    pending.current = true; setWorking(true); setItems(null); setSelected(new Set()); setPreview(null); setError(null); setBlocked(false);
    try {
      const rows = await api.catalogItems(undefined, undefined, language);
      if (alive.current) setItems(rows);
    } catch (e) {
      if (alive.current) setError(t("catalog.cleanup.failed", { message: api.errorMessage(e) }));
    } finally {
      pending.current = false;
      if (alive.current) setWorking(false);
    }
  }
  // Read only on explicit open; keep this snapshot and its language through the flow.
  useEffect(() => { void load(); }, []); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (!working && (preview || done || error)) statusRef.current?.focus(); }, [working, preview, done, error]);

  async function prepare() {
    if (pending.current || !selected.size) return;
    pending.current = true; setWorking(true); setError(null);
    try {
      const shown = await api.previewCatalogCleanup([...selected].sort());
      if (alive.current) setPreview(shown);
    } catch (e) {
      if (alive.current) setError(t("catalog.cleanup.failed", { message: api.errorMessage(e) }));
    } finally {
      pending.current = false;
      if (alive.current) setWorking(false);
    }
  }

  async function apply() {
    if (pending.current || !preview) return;
    pending.current = true; setWorking(true); setError(null);
    try {
      const result = await api.applyCatalogCleanup(preview.items.map((item) => item.id), preview.token);
      if (alive.current) { setDone(result); setPreview(null); }
      // The removal is confirmed; a failing refresh must not relabel it uncertain.
      // The owner is told even when navigation dismissed this view.
      try { await props.onRemoved(result); } catch { /* the owner reports its own refresh */ }
    } catch (e) {
      if (!alive.current) return;
      const failure = e as { status?: number };
      setPreview(null); setBlocked(true);
      if (api.isCatalogueCleanupStale(e)) setError(t("catalog.cleanup.stale"));
      else if (typeof failure?.status === "number" && failure.status < 500) setError(t("catalog.cleanup.rejected", { message: api.errorMessage(e) }));
      else setError(t("catalog.cleanup.uncertain", { message: api.errorMessage(e) }));
    } finally {
      pending.current = false;
      if (alive.current) setWorking(false);
    }
  }

  const needle = search.trim().toLowerCase();
  const texts = useMemo(() => new Map((items ?? []).map((item) => [item.id, searchText(item)])), [items]);
  const visible = useMemo(() => (items ?? []).filter((item) => !needle || texts.get(item.id)!.includes(needle)), [items, texts, needle]);
  const suggestions = useMemo(() => redundantCatalogPlacements(items ?? []), [items]);
  return <Overlay className="catalog-cleanup-dialog" labelledBy="catalog-cleanup-title" initialFocusRef={closeRef}
    onClose={() => { if (!pending.current) props.onClose(); }}>
    <h2 id="catalog-cleanup-title">{t("catalog.cleanup.title")}</h2>
    <p>{t("catalog.cleanup.retained")}</p>
    {error && <p className="field-error" role="alert" tabIndex={-1} ref={statusRef}>{error}</p>}
    {working && <p role="status">{t("catalog.cleanup.working")}</p>}
    {done ? <>
      <p role="status" tabIndex={-1} ref={statusRef}>{t("catalog.cleanup.done", { count: done.removedItemIds.length, backup: done.backup })}</p>
      <p>{t("catalog.cleanup.recovery")}</p>
    </> : blocked ? <button type="button" disabled={working} onClick={() => void load()}>{t("catalog.cleanup.reload")}</button>
      : preview ? <>
        <p role="status" tabIndex={-1} ref={statusRef}>{t("catalog.cleanup.nothingYet")}</p>
        <ul className="catalog-cleanup-list">{preview.items.map((item) => {
          // The list was read in the product language; the preview names the stored text.
          const shown = items?.find((row) => row.id === item.id) ?? item;
          return <li key={item.id}>
            <strong>{shown.name ?? item.id}{item.number ? ` (${item.number})` : ""}</strong><CatalogItemIdentity item={item} />
            {item.impact.lastPlacementForBinding && <p className="catalog-cleanup-warning">{t("catalog.cleanup.impact.last")}</p>}
            {item.impact.openProjectDevices > 0 && <p>{t("catalog.cleanup.impact.devices", { count: item.impact.openProjectDevices })}</p>}
          </li>;
        })}</ul>
        <p className="inspector-description">{t("catalog.cleanup.impact.unchecked")}</p>
      </> : <>
        <p>{t("catalog.cleanup.explanation")}</p>
        {items === null ? <p role="status">{t("catalog.cleanup.loading")}</p> : <>
          <input aria-label={t("catalog.cleanup.search")} placeholder={t("catalog.cleanup.search")} value={search} onChange={(e) => setSearch(e.target.value)} disabled={working} />
          <p>{t("catalog.cleanup.selection", { selected: selected.size, visible: visible.length, total: items.length })}</p>
          <div className="catalog-cleanup-actions catalog-cleanup-tools">
            <button type="button" disabled={working || !suggestions.length} onClick={() => setSelected(new Set(suggestions))}>{t("catalog.cleanup.suggest")}</button>
            <button type="button" disabled={working || !selected.size} onClick={() => setSelected(new Set())}>{t("catalog.cleanup.clear")}</button>
          </div>
          <p className="inspector-description">{t("catalog.cleanup.suggestHelp")}</p>
          {visible.length === 0 && <p>{t("catalog.cleanup.empty")}</p>}
          <ul className="catalog-cleanup-list">{visible.map((item) => <li key={item.id}>
            <label><input type="checkbox" value={item.id} checked={selected.has(item.id)} disabled={working}
              onChange={(e) => { const checked = e.target.checked; setSelected((old) => { const next = new Set(old); if (checked) next.add(item.id); else next.delete(item.id); return next; }); }} />
              <span><strong>{item.name ?? item.id}{item.number ? ` (${item.number})` : ""}</strong><CatalogItemIdentity item={item} /></span>
            </label>
          </li>)}</ul>
        </>}
      </>}
    <footer className="catalog-cleanup-actions">
      <button type="button" ref={closeRef} className="dialog-action secondary-action" disabled={working} onClick={props.onClose}>{done ? t("catalog.cleanup.close") : t("catalog.cleanup.cancel")}</button>
      {!done && !blocked && (preview ? <>
        <button type="button" className="dialog-action secondary-action" disabled={working} onClick={() => setPreview(null)}>{t("catalog.cleanup.back")}</button>
        <button type="button" className="dialog-action danger-action" disabled={working} onClick={() => void apply()}>{t("catalog.cleanup.confirm", { count: preview.items.length })}</button>
      </> : <button type="button" className="dialog-action" disabled={working || !selected.size || items === null} onClick={() => void prepare()}>{t("catalog.cleanup.preview")}</button>)}
    </footer>
  </Overlay>;
}
