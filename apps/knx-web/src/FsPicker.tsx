/** Modal file browser mounted imperatively as a promise-based stand-in for a native file picker. */
//! Modal file browser over `/api/fs/list` + `/api/fs/upload` for the web
//! build (no native OS picker in a browser). Mounted imperatively by
//! filePicker.ts's openMountPicker/saveMountPicker so callers can
//! `await` it exactly like @tauri-apps/plugin-dialog's open()/save().
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { noteRefusal } from "./api";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";
import { subscribeSessionExpired } from "./session";

interface Entry {
  name: string;
  is_dir: boolean;
}

interface Filter {
  name: string;
  extensions: string[];
}

function matchesFilter(name: string, filters: Filter[]): boolean {
  if (filters.length === 0) return true;
  const ext = name.split(".").pop()?.toLowerCase();
  return filters.some((f) => f.extensions.some((e) => e.toLowerCase() === ext));
}

// Both helpers below hold their own `fetch` — one needs a query string it
// builds itself, the other a `FormData` body `request()` would JSON-encode
// — so both must report a refusal by hand. `/api/fs/*` is behind the same
// guard as everything else (ADR-0026); without this, an expired session
// showed up as the word "authentication required" printed inside a file
// browser the user then could not leave.
async function listDir(path: string): Promise<Entry[]> {
  const url = `/api/fs/list?path=${encodeURIComponent(path)}`;
  const res = await fetch(url);
  if (!res.ok) {
    noteRefusal("/api/fs/list", res.status);
    throw new Error((await res.json().catch(() => null))?.error ?? res.statusText);
  }
  return res.json();
}

async function uploadFile(file: File): Promise<string> {
  const form = new FormData();
  form.append("file", file);
  const res = await fetch("/api/fs/upload", { method: "POST", body: form });
  if (!res.ok) {
    noteRefusal("/api/fs/upload", res.status);
    throw new Error((await res.json().catch(() => null))?.error ?? res.statusText);
  }
  return (await res.json()).path as string;
}

function Modal(props: {
  mode: "open" | "save";
  filters: Filter[];
  defaultName?: string;
  onResolve: (path: string | null) => void;
}) {
  const { mode, filters, defaultName, onResolve } = props;
  const t = useTranslate();
  const [dir, setDir] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const [name, setName] = useState(defaultName ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listDir(dir)
      .then(setEntries)
      .catch((e) => setError(String(e)));
  }, [dir]);

  return (
    <Overlay className="fs-picker" label={mode === "open" ? t("fsPicker.open") : t("fsPicker.saveAs")} onClose={() => onResolve(null)}>
        <h3>
          {mode === "open" ? t("fsPicker.open") : t("fsPicker.saveAs")} — /{dir}
        </h3>
        {error && <p className="field-error">{error}</p>}
        <ul className="fs-picker-list">
          {dir && <li><button onClick={() => setDir(dir.split("/").slice(0, -1).join("/"))}>..</button></li>}
          {entries
            .filter((e) => e.is_dir || mode === "save" || matchesFilter(e.name, filters))
            .map((e) => (
              <li
                key={e.name}>
                <button onClick={() => {
                  if (e.is_dir) setDir(dir ? `${dir}/${e.name}` : e.name);
                  else if (mode === "open") onResolve(dir ? `${dir}/${e.name}` : e.name);
                  else setName(e.name);
                }}
              >
                {e.name}{e.is_dir ? "/" : ""}
                </button>
              </li>
            ))}
        </ul>
        {mode === "save" && (
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t("fsPicker.filenamePlaceholder")} />
        )}
        <div className="fs-picker-actions">
          {mode === "open" && (
            <label className="fs-picker-upload">
              {t("fsPicker.upload")}
              <input
                type="file"
                className="fs-picker-upload-input"
                onChange={async (e) => {
                  const file = e.target.files?.[0];
                  if (!file) return;
                  try {
                    onResolve(await uploadFile(file));
                  } catch (err) {
                    setError(String(err));
                  }
                }}
              />
            </label>
          )}
          {mode === "save" && (
            <button
              onClick={() => onResolve(name ? (dir ? `${dir}/${name}` : name) : null)}
              disabled={!name}
            >
              {t("fsPicker.save")}
            </button>
          )}
          <button onClick={() => onResolve(null)}>{t("fsPicker.cancel")}</button>
        </div>
    </Overlay>
  );
}

function openDialog(mode: "open" | "save", filters: Filter[], defaultName?: string): Promise<string | null> {
  return new Promise((resolve) => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    let unsubscribe = () => {};
    function close(path: string | null) {
      unsubscribe();
      root.unmount();
      host.remove();
      resolve(path);
    }
    // This modal is mounted on its own root, a sibling of `#root` rather
    // than a descendant of `AuthGate`'s `.auth-gate-app`, so the `inert`
    // the gate puts on the workbench does not reach it: a 401 arriving
    // while the picker is open would raise the login screen over a dialog
    // that still held the focus trap. Closing it is the better of the two
    // answers — the user's next act is typing a password, and a file
    // listing fetched before the session ended is stale anyway. The caller
    // sees the same `null` it gets from Cancel, which every call site
    // already handles.
    unsubscribe = subscribeSessionExpired(() => close(null));
    root.render(<Modal mode={mode} filters={filters} defaultName={defaultName} onResolve={close} />);
  });
}

export function openMountPicker(filters: Filter[]): Promise<string | null> {
  return openDialog("open", filters);
}

export function saveMountPicker(defaultName: string): Promise<string | null> {
  return openDialog("save", [], defaultName);
}
