/** Manual sync, local search and verified acquisition through the shared job owner. */
import { useContext, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  onlineCatalogueApi as api,
  onlineError,
  type OnlineDownload,
  type OnlineEntry,
  type OnlinePage,
  type OnlineStatus,
} from "./onlineCatalogueApi";
import { feedOnlineCatalogueJob } from "./catalogueJobsApi";
import { CatalogueWizardOrigin, useOptionalCatalogueJobs } from "./CatalogueJobsContext";
import CatalogueJobSelection from "./CatalogueJobSelection";
import { OnlineCatalogueAccount } from "./OnlineCatalogueAccount";
import OnlineCatalogueError from "./OnlineCatalogueError";
import Overlay from "./Overlay";
import LegacyPasswordDialog from "./LegacyPasswordDialog";
import {
  legacyPasswordRefusal,
  type LegacyPasswordRefusal,
} from "./legacyInstall";
import { LegacyInstallReportView } from "./LegacyInstallReport";
import type { LegacyInstallReport } from "./api";
import { useTranslate } from "./i18n";
import "./OnlineCatalogue.css";

type LoginRequest = { entry: OnlineEntry; intent: number };
type PasswordPrompt = { entry: OnlineEntry; reason: LegacyPasswordRefusal };

/** Typed refusal thrown when the configured catalogue source is unavailable. */
class UnavailableSourceError extends Error {
  readonly body = { kind: "unavailable" } as const;

  constructor() {
    super("online catalogue source unavailable");
  }
}

export default function OnlineCatalogue({
  onInstalled,
}: {
  onInstalled: () => void | Promise<void>;
}) {
  const t = useTranslate();
  const jobs = useOptionalCatalogueJobs();
  const origin = useContext(CatalogueWizardOrigin);
  const [status, setStatus] = useState<OnlineStatus | null>(null);
  const [page, setPage] = useState<OnlinePage | null>(null);
  const [query, setQuery] = useState("");
  const [type, setType] = useState("");
  const [selected, setSelected] = useState<OnlineEntry | null>(null);
  const [busy, setBusy] = useState<string | null>("status");
  const [error, setError] = useState<ReturnType<typeof onlineError> | null>(null);
  const [syncResult, setSyncResult] = useState<"updated" | "notModified" | null>(
    null,
  );
  const [cancelling, setCancelling] = useState(false);
  const [account, setAccount] = useState(false);
  const [login, setLogin] = useState<LoginRequest | null>(null);
  const [prepare, setPrepare] = useState<OnlineEntry | null>(null);
  const [legacy, setLegacy] = useState<LegacyInstallReport | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState<PasswordPrompt | null>(
    null,
  );
  const mounted = useRef(true);
  const intent = useRef(0);
  const searchSerial = useRef(0);
  const lock = useRef(false);
  const lastSettled = useRef<OnlineDownload | null>(null);
  const lastRequested = useRef<string | null>(null);

  useEffect(() => {
    mounted.current = true;
    let current = true;
    void api
      .sources()
      .then((sources) => {
        if (
          !sources.available ||
          !sources.sources.some((source) => source.id === "reko")
        ) {
          throw new UnavailableSourceError();
        }
        return api.status();
      })
      .then((value) => {
        if (current) {
          setStatus(value);
          acceptDownload(value.download);
        }
      })
      .catch((caught) => {
        if (current) setError(onlineError(caught));
      })
      .finally(() => {
        if (current) setBusy(null);
      });
    return () => {
      current = false;
      mounted.current = false;
      intent.current += 1;
      searchSerial.current += 1;
    };
  }, []);

  const running = status?.download.state === "running";
  const occupied = Boolean(
    jobs &&
      (jobs.state.pending || jobs.state.status || jobs.state.uncertain),
  );

  function downloadEntry(
    download: OnlineDownload,
    page: OnlinePage | null,
    selected: OnlineEntry | null,
  ): OnlineEntry {
    const known =
      selected?.id === download.entryId
        ? selected
        : (page?.entries.find((entry) => entry.id === download.entryId) ??
          null);
    if (known) return known;
    return {
      id: download.entryId ?? "",
      manufacturer: "",
      manufacturerId: "",
      name: download.entryId ?? "",
      orderNumbers: [],
      fileName: download.entryId ?? "",
      fileType: "",
      size: download.totalBytes ?? 0,
      sha256: "",
      updated: "",
      programs: [],
      importable: false,
      downloaded: false,
    };
  }

  function acceptDownload(download: OnlineDownload) {
    setStatus((old) => (old ? { ...old, download } : old));
    if (download.state !== "running") setCancelling(false);
    if (download.state === "done") {
      setPage((old) =>
        old
          ? {
              ...old,
              entries: old.entries.map((entry) =>
                entry.id === download.entryId
                  ? { ...entry, downloaded: true }
                  : entry,
              ),
            }
          : old,
      );
      setSelected((old) =>
        old?.id === download.entryId ? { ...old, downloaded: true } : old,
      );
    }
    if (download.state === "failed" && download.error) {
      setError(download.error);
      const previous = lastSettled.current;
      lastSettled.current = download;
      const authFailure = [
        "needsLogin",
        "tokenRejected",
        "invalidCredentials",
      ].includes(download.error.kind);
      const startedHere =
        (previous?.state === "running" || previous === null) &&
        download.entryId === lastRequested.current;
      if (authFailure && startedHere && !login && !account) {
        // The user started this download, so a polled auth failure may open
        // the account dialog; a retained old failure on mount may not.
        setLogin({
          entry: downloadEntry(download, page, selected),
          intent: intent.current,
        });
        setAccount(true);
      }
    }
  }

  useEffect(() => {
    if (!running) return;
    let current = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await api.status();
        if (!current) return;
        setStatus(next);
        acceptDownload(next.download);
        if (next.download.state === "running") {
          timer = setTimeout(poll, 750);
        }
      } catch (caught) {
        // Only terminal status can settle Stop.
        if (current) setError(onlineError(caught));
      }
    };
    timer = setTimeout(poll, 750);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [running]);

  async function refresh() {
    if (lock.current) return;
    lock.current = true;
    setBusy("status");
    setError(null);
    try {
      const next = await api.status();
      if (mounted.current) {
        setStatus(next);
        acceptDownload(next.download);
      }
    } catch (caught) {
      if (mounted.current) setError(onlineError(caught));
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  async function search(offset = 0) {
    const serial = ++searchSerial.current;
    intent.current += 1;
    setSelected(null);
    setPrepare(null);
    setLogin(null);
    setLegacy(null);
    setError(null);
    setBusy("search");
    try {
      const result = await api.search(query, type, offset);
      if (mounted.current && serial === searchSerial.current) {
        setPage(result);
      }
    } catch (caught) {
      if (mounted.current && serial === searchSerial.current) {
        setError(onlineError(caught));
      }
    } finally {
      if (mounted.current && serial === searchSerial.current) setBusy(null);
    }
  }

  async function sync() {
    if (lock.current || running) return;
    lock.current = true;
    const serial = ++intent.current;
    ++searchSerial.current;
    setBusy("sync");
    setError(null);
    setSyncResult(null);
    setPrepare(null);
    setLogin(null);
    try {
      const result = await api.sync();
      if (mounted.current && serial === intent.current) {
        setStatus(result.status);
        setPage(null);
        setSelected(null);
        setSyncResult(result.outcome);
      }
    } catch (caught) {
      if (mounted.current && serial === intent.current) {
        setError(onlineError(caught));
      }
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  async function download(entry: OnlineEntry, serial = ++intent.current) {
    if (lock.current || running || !status) return;
    if (status.credentials.state !== "stored" && !entry.downloaded) {
      setLogin({ entry, intent: serial });
      setAccount(true);
      return;
    }
    await beginDownload(entry, serial);
  }

  async function beginDownload(entry: OnlineEntry, serial: number) {
    if (lock.current || !mounted.current || serial !== intent.current) return;
    lock.current = true;
    lastSettled.current = null;
    lastRequested.current = entry.id;
    setBusy("download");
    setError(null);
    setLogin(null);
    try {
      const next = await api.download(entry);
      if (mounted.current && serial === intent.current) {
        acceptDownload(next);
      }
    } catch (caught) {
      if (mounted.current && serial === intent.current) {
        // A direct refusal replaces whatever a retained download failure
        // showed before; the banner renders exactly this latest refusal.
        const refusal = onlineError(caught);
        setError(refusal);
        if (
          ["needsLogin", "tokenRejected", "invalidCredentials"].includes(
            refusal.kind,
          )
        ) {
          setLogin({ entry, intent: serial });
          setAccount(true);
        }
        void api
          .status()
          .then((next) => {
            if (mounted.current && serial === intent.current) setStatus(next);
          })
          .catch(() => {});
      }
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  async function cancel() {
    if (cancelling || lock.current) return;
    setCancelling(true);
    setError(null);
    try {
      const result = await api.cancel();
      if (mounted.current) acceptDownload(result);
    } catch (caught) {
      if (mounted.current) {
        // Stop remains pending until an explicit status read confirms settlement.
        setError(onlineError(caught));
      }
    }
  }

  async function installLegacy(
    entry: OnlineEntry,
    password?: string,
    remember = false,
  ) {
    if (lock.current) return;
    lock.current = true;
    const serial = ++intent.current;
    setBusy("legacy");
    setError(null);
    setPasswordPrompt(null);
    setLegacy(null);
    try {
      const report = await api.legacy(entry, password, remember);
      if (mounted.current && serial === intent.current) setLegacy(report);
      // A completed import invalidates the shared catalogue even if its
      // source view was replaced.
      await onInstalled();
    } catch (caught) {
      if (mounted.current && serial === intent.current) {
        const reason = legacyPasswordRefusal(caught);
        if (reason) setPasswordPrompt({ entry, reason });
        else setError(onlineError(caught));
      }
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  return (
    <section className="online-catalogue" aria-label={t("online.title")}>
      <p>{t("online.manualOnly")}</p>
      <div className="online-catalogue-actions">
        <button
          type="button"
          disabled={!status || !!busy || running || status.syncRunning}
          onClick={() => void sync()}
        >
          {busy === "sync" ? t("online.syncing") : t("online.sync")}
        </button>
        <button type="button" disabled={!!busy} onClick={() => void refresh()}>
          {t("online.refreshStatus")}
        </button>
        <button
          type="button"
          disabled={!status || !!busy || running || status.syncRunning}
          onClick={() => {
            setLogin(null);
            setAccount(true);
          }}
        >
          {t("online.account")}
        </button>
      </div>
      {busy === "status" && <p role="status">{t("online.loading")}</p>}
      {status && (
        <>
          {status.index ? (
            <dl className="online-catalogue-metadata">
              <dt>{t("online.lastChecked")}</dt>
              <dd>{status.index.lastCheckedAt}</dd>
              <dt>{t("online.indexDate")}</dt>
              <dd>{status.index.generatedAt}</dd>
              <dt>{t("online.entries")}</dt>
              <dd>{status.index.entryCount}</dd>
              {!!status.index.skippedEntries && (
                <>
                  <dt>{t("online.skipped")}</dt>
                  <dd>{status.index.skippedEntries}</dd>
                </>
              )}
            </dl>
          ) : (
            <p role="status">{t("online.empty")}</p>
          )}
          {status.indexProblem && (
            <p className="field-error">{t("online.indexProblem")}</p>
          )}
          {status.limit && <OnlineCatalogueError error={status.limit} />}
          {syncResult && (
            <p role="status">
              {t(
                syncResult === "notModified"
                  ? "online.notModified"
                  : "online.updated",
              )}
            </p>
          )}
          <form
            className="online-catalogue-search"
            onSubmit={(event) => {
              event.preventDefault();
              void search();
            }}
          >
            <label>
              {t("online.searchLabel")}
              <input
                value={query}
                maxLength={200}
                disabled={!!busy || running}
                onChange={(event) => {
                  setQuery(event.target.value);
                  setPage(null);
                  setSelected(null);
                  setPrepare(null);
                  intent.current += 1;
                  searchSerial.current += 1;
                }}
              />
            </label>
            <label>
              {t("online.fileType")}
              <select
                value={type}
                disabled={!!busy || running}
                onChange={(event) => {
                  setType(event.target.value);
                  setPage(null);
                  setSelected(null);
                  setPrepare(null);
                  intent.current += 1;
                  searchSerial.current += 1;
                }}
              >
                <option value="">{t("online.allTypes")}</option>
                {["knxprod", "vd1", "vd2", "vd3", "vd4", "vd5"].map(
                  (value) => (
                    <option key={value} value={value}>
                      {value.toUpperCase()}
                    </option>
                  ),
                )}
              </select>
            </label>
            <button
              type="submit"
              disabled={!status.index || !!busy || running}
            >
              {busy === "search" ? t("online.searching") : t("online.search")}
            </button>
          </form>
        </>
      )}
      {page && (
        <>
          <p role="status">
            {t("online.results", {
              total: page.total,
              from: page.total ? page.offset + 1 : 0,
              to: page.offset + page.entries.length,
            })}
          </p>
          <ul className="online-catalogue-results">
            {page.entries.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  aria-pressed={selected?.id === entry.id}
                  disabled={!!busy || running}
                  onClick={() => {
                    intent.current += 1;
                    setSelected(entry);
                    setPrepare(null);
                    setLegacy(null);
                  }}
                >
                  <strong>{entry.name}</strong>
                  <span>
                    {entry.manufacturer} · {entry.orderNumbers.join(", ")}
                  </span>
                  <span>
                    {entry.fileType.toUpperCase()} · {entry.fileName}
                  </span>
                  <span>
                    {!entry.importable
                      ? t("online.unsupported")
                      : entry.downloaded
                        ? t("online.downloaded")
                        : t("online.available")}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="online-catalogue-actions">
            <button
              type="button"
              disabled={!!busy || running || page.offset === 0}
              onClick={() => void search(Math.max(0, page.offset - page.limit))}
            >
              {t("online.previous")}
            </button>
            <button
              type="button"
              disabled={
                !!busy ||
                running ||
                page.offset + page.entries.length >= page.total
              }
              onClick={() => void search(page.offset + page.limit)}
            >
              {t("online.next")}
            </button>
          </div>
        </>
      )}
      {selected && (
        <div className="online-catalogue-details">
          <h3>{selected.name}</h3>
          <p>
            {selected.fileName} · {selected.size} B · {selected.updated}
          </p>
          <p className="online-catalogue-digest">SHA-256: {selected.sha256}</p>
          <h4>{t("online.programs")}</h4>
          {selected.programs.length ? (
            <ul>
              {selected.programs.map((program, index) => (
                <li key={index}>
                  <strong>{program.name}</strong> · {program.displayKey}
                  <div>
                    {t("online.programVersion", {
                      number: program.applicationNumber,
                      version: program.version,
                      mask: program.maskVersion,
                    })}
                  </div>
                  <div>{program.productNames.join(", ")}</div>
                </li>
              ))}
            </ul>
          ) : (
            <p>{t("online.noPrograms")}</p>
          )}
          {!selected.importable && <p>{t("online.unsupported")}</p>}
          <div className="online-catalogue-actions">
            <button
              type="button"
              disabled={
                !!busy ||
                running ||
                Boolean(status?.limit && !selected.downloaded)
              }
              onClick={() => void download(selected)}
            >
              {selected.downloaded
                ? t("online.verifyCached")
                : t("online.download")}
            </button>
            {selected.importable && selected.downloaded && (
              <button
                type="button"
                disabled={
                  !!busy ||
                  running ||
                  occupied ||
                  (selected.fileType === "knxprod" && !jobs)
                }
                onClick={() => {
                  if (selected.fileType === "knxprod") setPrepare(selected);
                  else void installLegacy(selected);
                }}
              >
                {t("online.import")}
              </button>
            )}
          </div>
          <p>{t("online.importBoundary")}</p>
        </div>
      )}
      {status && status.download.state !== "idle" && (
        <div className="online-catalogue-progress" role="status">
          <p>
            {t(
              running
                ? cancelling
                  ? "online.cancelling"
                  : "online.downloading"
                : status.download.state === "done"
                  ? "online.verified"
                  : "online.failed",
              { entry: status.download.entryId ?? "" },
            )}
          </p>
          {running && (
            <>
              <progress
                value={status.download.receivedBytes}
                max={status.download.totalBytes || undefined}
              />
              <span>
                {status.download.receivedBytes} /{" "}
                {status.download.totalBytes ?? "?"} B
              </span>
              <button
                type="button"
                disabled={cancelling}
                onClick={() => void cancel()}
              >
                {t("online.cancelDownload")}
              </button>
            </>
          )}
        </div>
      )}
      {prepare && jobs && (
        <CatalogueJobSelection
          file={{ name: prepare.fileName, size: prepare.size }}
          onDone={() => setPrepare(null)}
          onStart={(profile, ack) => {
            const entry = prepare;
            void jobs.controller.startSource(
              { name: entry.fileName, size: entry.size },
              profile,
              ack,
              origin,
              (owner) => feedOnlineCatalogueJob(owner, entry),
            );
          }}
        />
      )}
      {legacy && <LegacyInstallReportView report={legacy} />}
      {error && error.kind !== status?.limit?.kind && (
        <OnlineCatalogueError error={error} />
      )}
      {account &&
        status &&
        createPortal(
          <Overlay
            labelledBy="online-account-title"
            className="project-wizard online-catalogue-dialog"
            onClose={() => {
              intent.current += 1;
              setLogin(null);
              setAccount(false);
            }}
          >
            <h2 id="online-account-title">{t("online.account")}</h2>
            {login && (
              <p>{t("online.loginFor", { file: login.entry.fileName })}</p>
            )}
            <OnlineCatalogueAccount
              status={status}
              onSaved={(next) => {
                setStatus(next);
                setAccount(false);
                const pending = login;
                setLogin(null);
                if (pending && pending.intent === intent.current) {
                  void beginDownload(pending.entry, pending.intent);
                }
              }}
              onChanged={(next) => {
                intent.current += 1;
                setLogin(null);
                setStatus(next);
              }}
            />
            <button
              type="button"
              onClick={() => {
                intent.current += 1;
                setLogin(null);
                setAccount(false);
              }}
            >
              {t("online.close")}
            </button>
          </Overlay>,
          document.body,
        )}
      {passwordPrompt && (
        <LegacyPasswordDialog
          fileName={passwordPrompt.entry.fileName}
          reason={passwordPrompt.reason}
          onSubmit={(password, remember) =>
            void installLegacy(passwordPrompt.entry, password, remember)
          }
          onCancel={() => setPasswordPrompt(null)}
        />
      )}
    </section>
  );
}
