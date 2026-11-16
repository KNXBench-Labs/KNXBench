/** Shared installation-wide catalogue account form; secrets never enter settings or storage. */
import { useEffect, useRef, useState } from "react";
import {
  onlineCatalogueApi as api,
  onlineError,
  type OnlineStatus,
} from "./onlineCatalogueApi";
import { useTranslate } from "./i18n";
import OnlineCatalogueError from "./OnlineCatalogueError";
import "./OnlineCatalogue.css";

type AccountProps = {
  status: OnlineStatus;
  onSaved: (status: OnlineStatus) => void;
  onChanged?: (status: OnlineStatus) => void;
  configure?: boolean;
};

export function OnlineCatalogueAccount({
  status,
  onSaved,
  onChanged,
  configure = false,
}: AccountProps) {
  const t = useTranslate();
  const [mode, setMode] = useState<"password" | "token">("password");
  const [user, setUser] = useState("");
  const [password, setPassword] = useState("");
  const [token, setToken] = useState("");
  const [base, setBase] = useState(status.baseUrl);
  const [busy, setBusy] = useState(false);
  const locked = useRef(false);
  const [error, setError] = useState<ReturnType<typeof onlineError> | null>(null);
  const [saved, setSaved] = useState(false);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  useEffect(() => {
    setBase(status.baseUrl);
    setUser("");
    setPassword("");
    setToken("");
  }, [status.baseUrl]);

  const blocked =
    busy || status.syncRunning || status.download.state === "running";

  async function write(action: () => Promise<OnlineStatus>, account: boolean) {
    if (locked.current || blocked) return;
    locked.current = true;
    setBusy(true);
    setError(null);
    setSaved(false);
    // Clear sensitive drafts on every submission, including failed saves;
    // never read them back.
    setPassword("");
    setToken("");
    setUser("");
    try {
      await action();
      const next = await api.status();
      if (alive.current) {
        setSaved(true);
        if (account) onSaved(next);
        else onChanged?.(next);
      }
    } catch (caught) {
      if (alive.current) setError(onlineError(caught));
    } finally {
      locked.current = false;
      if (alive.current) setBusy(false);
    }
  }

  const credentialStateKey =
    status.credentials.state === "stored"
      ? "online.credentialsStored"
      : status.credentials.state === "empty"
        ? "online.credentialsEmpty"
        : "online.credentialsProblem";

  return (
    <div className="online-catalogue-account">
      <p>{t("online.sharedAccount")}</p>
      <p>
        {t("online.accountStatus", {
          state: t(credentialStateKey),
          user: status.credentials.user ?? "—",
        })}
      </p>
      {status.credentials.problem && (
        <p className="field-error">{t("online.secretProblem")}</p>
      )}
      <a href={status.baseUrl} target="_blank" rel="noopener noreferrer">
        {t("online.registration")}
      </a>
      {configure && (
        <form
          onSubmit={(event) => {
            event.preventDefault();
            const value = base.trim();
            void write(() => api.configure(value || null), false);
          }}
        >
          <label>
            {t("online.baseUrl")}
            <input
              type="url"
              required
              value={base}
              disabled={blocked}
              onChange={(event) => setBase(event.target.value)}
            />
          </label>
          <p>{t("online.originWarning")}</p>
          <div className="online-catalogue-actions">
            <button
              type="submit"
              disabled={blocked || !base.startsWith("https://")}
            >
              {t("online.saveAddress")}
            </button>
            <button
              type="button"
              disabled={blocked || status.baseUrlIsDefault}
              onClick={() => void write(() => api.configure(null), false)}
            >
              {t("online.defaultAddress")}
            </button>
          </div>
        </form>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const credentials =
            mode === "password"
              ? ({ mode, user, password } as const)
              : ({ mode, token } as const);
          void write(() => api.credentials(credentials), true);
        }}
      >
        <label>
          {t("online.authMode")}
          <select
            value={mode}
            disabled={blocked}
            onChange={(event) => {
              setMode(event.target.value as typeof mode);
              setPassword("");
              setToken("");
              setUser("");
            }}
          >
            <option value="password">{t("online.passwordMode")}</option>
            <option value="token">{t("online.tokenMode")}</option>
          </select>
        </label>
        {mode === "password" ? (
          <>
            <label>
              {t("online.user")}
              <input
                autoComplete="off"
                value={user}
                disabled={blocked}
                required
                maxLength={256}
                onChange={(event) => setUser(event.target.value)}
              />
            </label>
            <label>
              {t("online.password")}
              <input
                type="password"
                autoComplete="new-password"
                value={password}
                disabled={blocked}
                required
                maxLength={1024}
                onChange={(event) => setPassword(event.target.value)}
              />
            </label>
          </>
        ) : (
          <label>
            {t("online.token")}
            <input
              type="password"
              autoComplete="off"
              value={token}
              disabled={blocked}
              required
              maxLength={4096}
              onChange={(event) => setToken(event.target.value)}
            />
          </label>
        )}
        <div className="online-catalogue-actions">
          <button
            type="submit"
            disabled={
              blocked || (mode === "password" ? !user || !password : !token)
            }
          >
            {busy ? t("online.saving") : t("online.saveAccount")}
          </button>
          <button
            type="button"
            disabled={blocked || status.credentials.state === "empty"}
            onClick={() => void write(api.forget, false)}
          >
            {t("online.forget")}
          </button>
        </div>
      </form>
      {saved && <p role="status">{t("online.saved")}</p>}
      {error && <OnlineCatalogueError error={error} />}
    </div>
  );
}

/** Lazy settings entry avoids even own-backend requests until opened. */
export default function OnlineCatalogueSettings() {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<OnlineStatus | null>(null);
  const [error, setError] = useState<ReturnType<typeof onlineError> | null>(null);

  useEffect(() => {
    if (!open) return;
    let current = true;
    void api
      .status()
      .then((value) => {
        if (current) setStatus(value);
      })
      .catch((caught) => {
        if (current) setError(onlineError(caught));
      });
    return () => {
      current = false;
    };
  }, [open]);

  return (
    <section className="settings-section online-catalogue-settings">
      <h2 id="settings-catalogues" tabIndex={-1}>
        {t("online.settings")}
      </h2>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        {t("online.account")}
      </button>
      {open &&
        (status ? (
          <OnlineCatalogueAccount
            status={status}
            onSaved={setStatus}
            onChanged={setStatus}
            configure
          />
        ) : error ? (
          <OnlineCatalogueError error={error} />
        ) : (
          <p role="status">{t("online.loading")}</p>
        ))}
    </section>
  );
}
