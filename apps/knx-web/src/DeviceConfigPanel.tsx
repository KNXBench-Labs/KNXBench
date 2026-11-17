/** Device configuration profile panel: structured raw JSON editing per device. */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";
import { snapshotContextFromTree } from "./snapshotIdentity";
import { createDevconfClient, type DevconfAction, type DevconfPreview, type DevconfProfileView, type DevconfView } from "./features/devconf/client";

export interface DeviceConfigPanelProps {
  deviceId: number;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void;
}

const MAX_DOCUMENT_TEXT = 1024 * 1024;

function encodeB64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export default function DeviceConfigPanel({ deviceId, tree, onApplied }: DeviceConfigPanelProps) {
  const t = useTranslate();
  const client = useMemo(() => createDevconfClient(), []);
  const context = snapshotContextFromTree(tree);
  const [view, setView] = useState<DevconfView | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [raw, setRaw] = useState("");
  const [moduleId, setModuleId] = useState("");
  const [moduleVersion, setModuleVersion] = useState("");
  const [moduleSha256, setModuleSha256] = useState("");
  const [draftError, setDraftError] = useState<string | null>(null);
  const [pending, setPending] = useState<DevconfPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const current: DevconfProfileView | null = useMemo(
    () => view?.profiles.find((p) => p.device === deviceId) ?? null,
    [view, deviceId],
  );

  const reload = useCallback(async (signal?: AbortSignal) => {
    try {
      const next = await client.read(signal);
      setView(next);
      setLoadError(null);
      const profile = next.profiles.find((p) => p.device === deviceId) ?? null;
      if (profile) {
        setModuleId(profile.moduleId);
        setModuleVersion(profile.moduleVersion);
        setModuleSha256(profile.moduleSha256);
      }
      return next;
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return null;
      setLoadError(e instanceof Error ? e.message : String(e));
      return null;
    }
  }, [client, deviceId]);

  useEffect(() => {
    const controller = new AbortController();
    void reload(controller.signal);
    return () => controller.abort();
  }, [reload]);

  // Keep the raw editor in sync with the stored document when the panel
  // (re)loads or the applied tree changed underneath.
  // Reset the raw editor whenever a different stored profile identity loads.
  const loadedKey = current ? `${current.moduleSha256}:${current.documentBytes}` : "none";
  const lastLoaded = useRef(loadedKey);
  useEffect(() => {
    if (lastLoaded.current !== loadedKey) {
      lastLoaded.current = loadedKey;
      setRaw("");
    }
  }, [loadedKey]);

  const canEdit = context !== null;

  const preview = async () => {
    if (!context) return;
    setError(null); setPending(null);
    let documentText = raw;
    if (documentText.length > MAX_DOCUMENT_TEXT) {
      setDraftError(t("deviceConfig.documentTooLarge"));
      return;
    }
    try { JSON.parse(documentText); } catch {
      setDraftError(t("deviceConfig.invalidJson"));
      return;
    }
    setDraftError(null);
    setBusy(true);
    try {
      const bytes = new TextEncoder().encode(documentText);
      const action: DevconfAction = current
        ? { kind: "set", draft: { device: deviceId, moduleId, moduleVersion, moduleSha256, documentB64: encodeB64(bytes) } }
        : { kind: "set", draft: { device: deviceId, moduleId, moduleVersion, moduleSha256, documentB64: encodeB64(bytes) } };
      const result = await client.preview(context, action);
      setPending(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    if (!pending) return;
    setBusy(true); setError(null);
    try {
      const applied = await client.apply(pending.token);
      if (applied.changed) {
        // The server bumped the project revision; the caller refreshes the
        // tree, which re-keys this panel's context.
        onApplied(tree);
      }
      setPending(null);
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPending(null);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!context || !current) return;
    setBusy(true); setError(null);
    try {
      const action: DevconfAction = { kind: "remove", device: deviceId };
      const result = await client.preview(context, action);
      const applied = await client.apply(result.token);
      if (applied.changed) onApplied(tree);
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  if (!canEdit) {
    return <p role="status">{t("deviceConfig.unavailable")}</p>;
  }
  if (loadError) {
    return <p role="alert">{loadError}</p>;
  }

  return (
    <div className="device-config-panel">
      <h3>{t("deviceConfig.heading")}</h3>
      {current ? (
        <p className="device-config-current">
          {t("deviceConfig.current", {
            module: current.moduleId,
            version: current.moduleVersion,
            bytes: String(current.documentBytes),
          })}
        </p>
      ) : (
        <p className="device-config-current">{t("deviceConfig.none")}</p>
      )}
      <label>
        {t("deviceConfig.moduleId")}
        <input value={moduleId} onChange={(e) => setModuleId(e.target.value)} maxLength={64} />
      </label>
      <label>
        {t("deviceConfig.moduleVersion")}
        <input value={moduleVersion} onChange={(e) => setModuleVersion(e.target.value)} maxLength={24} />
      </label>
      <label>
        {t("deviceConfig.moduleSha256")}
        <input value={moduleSha256} onChange={(e) => setModuleSha256(e.target.value)} maxLength={64} pattern="[0-9a-f]{64}" />
      </label>
      <label>
        {t("deviceConfig.document")}
        <textarea value={raw} onChange={(e) => setRaw(e.target.value)} rows={12} spellCheck={false} />
      </label>
      {draftError && <p role="alert">{draftError}</p>}
      {error && <p role="alert">{error}</p>}
      <div className="device-config-actions">
        <button type="button" disabled={busy || !moduleId || !moduleVersion || !moduleSha256} onClick={() => void preview()}>
          {t("deviceConfig.preview")}
        </button>
        {current && (
          <button type="button" disabled={busy} onClick={() => void remove()}>
            {t("deviceConfig.remove")}
          </button>
        )}
      </div>
      {pending && (
        <div className="device-config-confirm" role="group" aria-label={t("deviceConfig.confirmLabel")}>
          <p>{pending.noOp ? t("deviceConfig.noChange") : t("deviceConfig.confirmText")}</p>
          {!pending.noOp && (
            <button type="button" disabled={busy} onClick={() => void apply()}>
              {t("deviceConfig.apply")}
            </button>
          )}
          <button type="button" disabled={busy} onClick={() => { setPending(null); void client.cancel(pending.token).catch(() => undefined); }}>
            {t("deviceConfig.cancel")}
          </button>
        </div>
      )}
    </div>
  );
}
