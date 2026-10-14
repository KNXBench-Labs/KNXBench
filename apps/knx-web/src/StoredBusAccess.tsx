/** Stored ETS bus access and IP configuration: read-only, values hidden until asked (Goal B3). */
import { useCallback, useEffect, useId, useRef, useState } from "react";
import * as api from "./api";
import type { StoredRecord, StoredRecordField } from "./api";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import Overlay from "./Overlay";
import "./storedBusAccess.css";

type Load = { state: "loading" } | { state: "failed"; message: string } | { state: "ready"; records: StoredRecord[] };

const CLASS_KEYS: Record<NonNullable<StoredRecordField["hidden"]>["class"], MessageKey> = {
  empty: "storedBusAccess.class.empty", ipv4Address: "storedBusAccess.class.ipv4Address",
  macAddress: "storedBusAccess.class.macAddress", guid: "storedBusAccess.class.guid",
  number: "storedBusAccess.class.number", text: "storedBusAccess.class.text",
};
const REASON_KEYS: Record<NonNullable<StoredRecord["owner"]["reason"]>, MessageKey> = {
  notKeyed: "storedBusAccess.owner.notKeyed", noMatch: "storedBusAccess.owner.noMatch",
  severalMatches: "storedBusAccess.owner.severalMatches",
};
const ELEMENT_KEYS: Record<string, MessageKey> = {
  BusAccess: "storedBusAccess.element.BusAccess", IPConfig: "storedBusAccess.element.IPConfig",
};
const PARAMETER_NOT_SPLIT = "parameterNotSplit";

/** The project's stored records; `reload` reads them again (after a change). */
function useStoredRecords() {
  const [load, setLoad] = useState<Load>({ state: "loading" });
  const live = useRef(true);
  const reload = useCallback(async () => {
    try {
      const view = await api.storedBusAccess();
      if (live.current) setLoad({ state: "ready", records: view.records });
    } catch (e) {
      if (live.current) setLoad({ state: "failed", message: api.errorMessage(e) });
    }
  }, []);
  useEffect(() => {
    live.current = true;
    void reload();
    return () => { live.current = false; };
  }, [reload]);
  return { load, reload };
}

function ownerText(owner: StoredRecord["owner"], t: Translate): string {
  const values = { address: owner.address ?? "–", name: owner.name ?? "" };
  if (owner.kind === "line") return t("storedBusAccess.owner.line", values).trim();
  if (owner.kind === "device") return t("storedBusAccess.owner.device", values).trim();
  return t(REASON_KEYS[owner.reason ?? "notKeyed"]);
}

function FieldValue({ field }: { field: StoredRecordField }) {
  const t = useTranslate();
  if (field.value !== null) return <code className="stored-bus-access-value">{field.value}</code>;
  const hidden = field.hidden ?? { class: "text" as const, length: 0 };
  return <span className="stored-bus-access-hidden">
    {t("storedBusAccess.hidden", { count: hidden.length, class: t(CLASS_KEYS[hidden.class]) })}</span>;
}

function RecordView({ record, onChanged }: { record: StoredRecord; onChanged: () => void }) {
  const t = useTranslate();
  const [revealed, setRevealed] = useState<StoredRecord | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const shown = revealed ?? record;
  const anyHidden = record.fields.some(field => field.value === null);
  async function reveal() {
    setBusy(true); setError(null);
    try {
      setRevealed(await api.revealStoredBusAccess(record.recordId));
    } catch (e) {
      if (api.isStoredBusAccessChanged(e)) onChanged();
      else setError(t("storedBusAccess.revealFailed"));
    } finally {
      setBusy(false);
    }
  }
  const unread = record.unread.filter(name => name !== PARAMETER_NOT_SPLIT);
  return <article className="stored-bus-access-record" data-record-id={record.recordId} data-revealed={revealed !== null}>
    <header>
      <h3>{t(ELEMENT_KEYS[record.element] ?? "storedBusAccess.element.other", { element: record.element })}</h3>
      <p className="stored-bus-access-owner" data-owner={record.owner.kind}>{ownerText(record.owner, t)}</p>
      {record.status === "read" && anyHidden && (revealed
        ? <button type="button" onClick={() => setRevealed(null)}>{t("storedBusAccess.hide")}</button>
        : <button type="button" disabled={busy} onClick={() => void reveal()}>{t("storedBusAccess.show")}</button>)}
    </header>
    {record.status === "tooLarge" && <p className="stored-bus-access-status">{t("storedBusAccess.status.tooLarge")}</p>}
    {record.status === "notWellFormed" && <p className="stored-bus-access-status">{t("storedBusAccess.status.notWellFormed")}</p>}
    {error && <p className="field-error" role="alert">{error}</p>}
    {shown.fields.length > 0 && <dl className="stored-bus-access-fields">{shown.fields.map((field, index) =>
      <div key={`${field.part}-${field.name}-${index}`}>
        <dt>{field.name}
          {field.part === "parameter" && <small>{t("storedBusAccess.part.parameter")}</small>}
          {!field.known && <small>{t("storedBusAccess.unknownField")}</small>}</dt>
        <dd><FieldValue field={field} /></dd>
      </div>)}</dl>}
    {record.unread.includes(PARAMETER_NOT_SPLIT) && <p className="stored-bus-access-note">{t("storedBusAccess.parameterNotSplit")}</p>}
    {unread.length > 0 && <p className="stored-bus-access-note">{t("storedBusAccess.unread", { names: unread.join(", ") })}</p>}
    <details className="stored-bus-access-source">
      <summary>{t("storedBusAccess.source")}</summary>
      <dl>
        <div><dt>{t("storedBusAccess.sourceFile")}</dt><dd><code>{record.source.sourcePath}</code></dd></div>
        <div><dt>{t("storedBusAccess.sourcePath")}</dt><dd><code>{record.source.xpath}</code></dd></div>
        <div><dt>{t("storedBusAccess.sourceSize")}</dt><dd>{t("storedBusAccess.bytes", { count: record.source.size })}</dd></div>
        <div><dt>{t("storedBusAccess.sourceSha256")}</dt><dd><code>{record.source.sha256}</code></dd></div>
      </dl>
    </details>
  </article>;
}

function RecordList({ records, onChanged }: { records: StoredRecord[]; onChanged: () => void }) {
  return <div className="stored-bus-access-records">
    {records.map(record => <RecordView key={record.recordId} record={record} onChanged={onChanged} />)}
  </div>;
}

/** Every stored record of the open project, from the command palette. */
export function StoredBusAccessDialog({ onClose }: { onClose: () => void }) {
  const t = useTranslate();
  const { load, reload } = useStoredRecords();
  const [changed, setChanged] = useState(false);
  const changedAndReload = () => { setChanged(true); void reload(); };
  return <Overlay className="stored-bus-access-panel" labelledBy="stored-bus-access-title" onClose={onClose}>
    <header className="stored-bus-access-heading">
      <h2 id="stored-bus-access-title">{t("storedBusAccess.title")}</h2>
      <button type="button" onClick={onClose}>{t("storedBusAccess.close")}</button>
    </header>
    <p className="stored-bus-access-intro">{t("storedBusAccess.intro")}</p>
    {changed && <p role="status" className="stored-bus-access-notice">{t("storedBusAccess.changed")}</p>}
    {load.state === "loading" && <p role="status">{t("storedBusAccess.loading")}</p>}
    {load.state === "failed" && <p role="alert" className="field-error">{t("storedBusAccess.failed")} {load.message}</p>}
    {load.state === "ready" && (load.records.length === 0
      ? <p className="stored-bus-access-empty">{t("storedBusAccess.empty")}</p>
      : <RecordList records={load.records} onChanged={changedAndReload} />)}
  </Overlay>;
}

/** The line inspector's share: this line's records, or nothing at all. */
export function LineStoredBusAccess({ lineId }: { lineId: number }) {
  const t = useTranslate();
  // The properties panel and the structure workspace can show one line twice.
  const headingId = useId();
  const { load, reload } = useStoredRecords();
  if (load.state === "failed") return <p className="field-error">{t("storedBusAccess.failed")}</p>;
  if (load.state !== "ready") return null;
  const records = load.records.filter(record => record.owner.kind === "line" && record.owner.id === lineId);
  if (records.length === 0) return null;
  return <section className="stored-bus-access-line" aria-labelledby={headingId}>
    <h3 id={headingId}>{t("storedBusAccess.lineTitle")}</h3>
    <p className="stored-bus-access-intro">{t("storedBusAccess.intro")}</p>
    <RecordList records={records} onChanged={() => void reload()} />
  </section>;
}
