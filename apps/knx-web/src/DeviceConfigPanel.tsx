/** Device configuration profile panel: loads and edits one device's stored raw JSON document. */
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";
import { snapshotContextFromTree, type SnapshotContext } from "./snapshotIdentity";
import { createDevconfClient, type DevconfAction, type DevconfDeviceView, type DevconfPreview } from "./features/devconf/client";

export interface DeviceConfigPanelProps {
  deviceId: number;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void | Promise<void>;
}

const MAX_DOCUMENT_TEXT = 1024 * 1024;

function encodeB64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function decodeB64(value: string): Uint8Array {
  const binary = atob(value.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

/** What the server stored when it was last read: the base a draft is compared against. */
interface Baseline {
  /** Content identity of the stored document (its SHA-256) plus module identity; `none` without a profile. */
  identity: string;
  exists: boolean;
  moduleId: string;
  moduleVersion: string;
  moduleSha256: string;
  bytes: number;
  /** Exact stored bytes (base64url), sent back unchanged while the text is not edited. */
  b64: string;
  text: string;
  /** The stored bytes are not UTF-8 and cannot be shown as text. */
  undecodable: boolean;
  /** Why the stored bytes would not be admitted as a new document (`null`: strict JSON). */
  documentError: string | null;
}

interface Draft { raw: string; moduleId: string; moduleVersion: string; moduleSha256: string }

function baselineFrom(view: DevconfDeviceView): Baseline {
  const profile = view.profile;
  if (!profile) {
    return { identity: "none", exists: false, moduleId: "", moduleVersion: "", moduleSha256: "", bytes: 0, b64: "", text: "", undecodable: false, documentError: null };
  }
  let text = "";
  let undecodable = false;
  try {
    text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(decodeB64(profile.documentB64));
  } catch {
    undecodable = true;
  }
  return {
    identity: `${profile.documentSha256}|${profile.moduleId}|${profile.moduleVersion}|${profile.moduleSha256}`,
    exists: true,
    moduleId: profile.moduleId,
    moduleVersion: profile.moduleVersion,
    moduleSha256: profile.moduleSha256,
    bytes: profile.documentBytes,
    b64: profile.documentB64,
    text,
    undecodable,
    documentError: profile.documentError,
  };
}

function draftOf(base: Baseline): Draft {
  return { raw: base.text, moduleId: base.moduleId, moduleVersion: base.moduleVersion, moduleSha256: base.moduleSha256 };
}

function isDirty(base: Baseline, draft: Draft): boolean {
  return draft.raw !== base.text || draft.moduleId !== base.moduleId || draft.moduleVersion !== base.moduleVersion || draft.moduleSha256 !== base.moduleSha256;
}

const sameContext = (a: SnapshotContext, b: SnapshotContext) =>
  a.serverIncarnation === b.serverIncarnation && a.projectIncarnation === b.projectIncarnation && a.snapshotRevision === b.snapshotRevision;

/**
 * Draft policy (WP-48.4): a clean editor reloads whenever the project revision changes;
 * a dirty draft is never overwritten. If the stored content changed underneath it, the
 * draft is kept, previews are blocked and the user chooses to keep the draft or load
 * the stored version. State is keyed by server, project incarnation and device, so
 * equal numeric device ids in another project never share a draft.
 */
export default function DeviceConfigPanel({ deviceId, tree, onApplied }: DeviceConfigPanelProps) {
  const t = useTranslate();
  const context = snapshotContextFromTree(tree);
  if (!context) return <p role="status">{t("deviceConfig.unavailable")}</p>;
  return <DeviceConfigEditor key={`${context.serverIncarnation}:${context.projectIncarnation}:${deviceId}`} deviceId={deviceId} context={context} onApplied={onApplied} />;
}

function DeviceConfigEditor({ deviceId, context, onApplied }: { deviceId: number; context: SnapshotContext; onApplied: DeviceConfigPanelProps["onApplied"] }) {
  const t = useTranslate();
  const client = useMemo(() => createDevconfClient(), []);
  const [baseline, setBaseline] = useState<Baseline | null>(null);
  const [incoming, setIncoming] = useState<Baseline | null>(null);
  const [draft, setDraft] = useState<Draft>({ raw: "", moduleId: "", moduleVersion: "", moduleSha256: "" });
  const [outdated, setOutdated] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [draftError, setDraftError] = useState<string | null>(null);
  const [pending, setPending] = useState<DevconfPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { serverIncarnation, projectIncarnation, snapshotRevision } = context;
  const latest = useRef({ baseline, draft, context, pending });
  latest.current = { baseline, draft, context, pending };
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);

  const clearPending = () => {
    const token = latest.current.pending?.token;
    setPending(null);
    if (token) void client.cancel(token).catch(() => undefined);
  };

  // Every revision of this project/device reads the stored document again. A response is
  // used only if it carries exactly the context it was requested for; anything else
  // (late, or from a different server/project/revision) is dropped.
  useEffect(() => {
    const controller = new AbortController();
    const requested: SnapshotContext = { serverIncarnation, projectIncarnation, snapshotRevision };
    if (latest.current.pending) clearPending();
    client.readDevice(deviceId, controller.signal).then((view) => {
      if (controller.signal.aborted || !mounted.current) return;
      if (view.device !== deviceId || !sameContext(view, requested)) { setOutdated(true); return; }
      setOutdated(false);
      setLoadError(null);
      const next = baselineFrom(view);
      const { baseline: base, draft: current } = latest.current;
      if (!base || !isDirty(base, current)) {
        setBaseline(next); setDraft(draftOf(next)); setIncoming(null); setDraftError(null);
      } else if (base.identity === next.identity) {
        setBaseline(next); setIncoming(null);
      } else {
        setIncoming(next);
      }
    }, (e: unknown) => {
      if (controller.signal.aborted || (e instanceof DOMException && e.name === "AbortError")) return;
      setLoadError(e instanceof Error ? e.message : String(e));
    });
    return () => controller.abort();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, deviceId, serverIncarnation, projectIncarnation, snapshotRevision]);

  const edit = (change: Partial<Draft>) => { if (pending) clearPending(); setDraft((d) => ({ ...d, ...change })); };

  const refreshProject = async () => {
    try { await onApplied(await api.currentProject()); } catch (e) { if (mounted.current) setError(e instanceof Error ? e.message : String(e)); }
  };

  const preview = async () => {
    if (!baseline || incoming) return;
    setError(null); clearPending();
    const text = draft.raw;
    if (text.length > MAX_DOCUMENT_TEXT) { setDraftError(t("deviceConfig.documentTooLarge")); return; }
    try { JSON.parse(text); } catch { setDraftError(t("deviceConfig.invalidJson")); return; }
    setDraftError(null);
    setBusy(true);
    try {
      // Unedited text goes back as the exact stored bytes (textarea newline normalisation must not alter them).
      const documentB64 = baseline.exists && !baseline.undecodable && text === baseline.text ? baseline.b64 : encodeB64(new TextEncoder().encode(text));
      const action: DevconfAction = { kind: "set", draft: { device: deviceId, moduleId: draft.moduleId, moduleVersion: draft.moduleVersion, moduleSha256: draft.moduleSha256, documentB64 } };
      const result = await client.preview(context, action);
      if (!mounted.current || !sameContext(latest.current.context, context)) { void client.cancel(result.token).catch(() => undefined); return; }
      setPending(result);
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (mounted.current) setBusy(false);
    }
  };

  const apply = async () => {
    if (!pending) return;
    setBusy(true); setError(null);
    try {
      await client.apply(pending.token);
      if (mounted.current) setPending(null);
      await refreshProject();
    } catch (e) {
      if (mounted.current) { setError(e instanceof Error ? e.message : String(e)); setPending(null); }
    } finally {
      if (mounted.current) setBusy(false);
    }
  };

  const remove = async () => {
    if (!baseline?.exists || incoming) return;
    setBusy(true); setError(null);
    try {
      const result = await client.preview(context, { kind: "remove", device: deviceId });
      await client.apply(result.token);
      await refreshProject();
    } catch (e) {
      if (mounted.current) setError(e instanceof Error ? e.message : String(e));
    } finally {
      if (mounted.current) setBusy(false);
    }
  };

  if (loadError) return <p role="alert">{loadError}</p>;
  if (!baseline) return <p role="status">{t("deviceConfig.loading")}</p>;

  const dirty = isDirty(baseline, draft);
  return (
    <div className="device-config-panel">
      <h3>{t("deviceConfig.heading")}</h3>
      {baseline.exists ? (
        <p className="device-config-current">
          {t("deviceConfig.current", { module: baseline.moduleId, version: baseline.moduleVersion, bytes: String(baseline.bytes) })}
        </p>
      ) : (
        <p className="device-config-current">{t("deviceConfig.none")}</p>
      )}
      {baseline.undecodable && <p className="device-config-notice" role="status">{t("deviceConfig.notUtf8")}</p>}
      {baseline.documentError && !baseline.undecodable && <p className="device-config-notice" role="status">{t("deviceConfig.storedNotStrict", { reason: baseline.documentError })}</p>}
      {outdated && (
        <p className="device-config-outdated" role="status">
          {t("deviceConfig.outdated")}{" "}
          <button type="button" onClick={() => void refreshProject()}>{t("deviceConfig.refreshProject")}</button>
        </p>
      )}
      {incoming && (
        <div className="device-config-stale" role="alert">
          <p>{t("deviceConfig.staleDraft")}</p>
          <button type="button" onClick={() => { setBaseline(incoming); setIncoming(null); }}>{t("deviceConfig.keepDraft")}</button>
          <button type="button" onClick={() => { setBaseline(incoming); setDraft(draftOf(incoming)); setIncoming(null); setDraftError(null); }}>{t("deviceConfig.loadStored")}</button>
        </div>
      )}
      <label>
        {t("deviceConfig.moduleId")}
        <input value={draft.moduleId} onChange={(e) => edit({ moduleId: e.target.value })} maxLength={64} />
      </label>
      <label>
        {t("deviceConfig.moduleVersion")}
        <input value={draft.moduleVersion} onChange={(e) => edit({ moduleVersion: e.target.value })} maxLength={24} />
      </label>
      <label>
        {t("deviceConfig.moduleSha256")}
        <input value={draft.moduleSha256} onChange={(e) => edit({ moduleSha256: e.target.value })} maxLength={64} pattern="[0-9a-f]{64}" />
      </label>
      <label>
        {t("deviceConfig.document")}
        <textarea value={draft.raw} onChange={(e) => edit({ raw: e.target.value })} rows={12} spellCheck={false} />
      </label>
      {draftError && <p role="alert">{draftError}</p>}
      {error && <p role="alert">{error}</p>}
      <div className="device-config-actions">
        <button type="button" disabled={busy || incoming !== null || !draft.moduleId || !draft.moduleVersion || !draft.moduleSha256} onClick={() => void preview()}>
          {t("deviceConfig.preview")}
        </button>
        {baseline.exists && (
          <button type="button" disabled={busy || incoming !== null} onClick={() => void remove()}>
            {t("deviceConfig.remove")}
          </button>
        )}
        {dirty && <span className="device-config-dirty">{t("deviceConfig.unsaved")}</span>}
      </div>
      {pending && (
        <div className="device-config-confirm" role="group" aria-label={t("deviceConfig.confirmLabel")}>
          <p>{pending.noOp ? t("deviceConfig.noChange") : t("deviceConfig.confirmText")}</p>
          {!pending.noOp && (
            <button type="button" disabled={busy} onClick={() => void apply()}>
              {t("deviceConfig.apply")}
            </button>
          )}
          <button type="button" disabled={busy} onClick={() => clearPending()}>
            {t("deviceConfig.cancel")}
          </button>
        </div>
      )}
    </div>
  );
}
