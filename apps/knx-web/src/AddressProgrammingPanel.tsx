/** Programming an individual address: wait for exactly one pressed button, then MP §2.3. */
// Not a download (docs/GLOSSARY.md): this writes only the device's
// individual address, to whichever single device is in programming mode.
// The panel decides nothing: the server reports its recovery gate before
// consent, then derives the phrase for the new address and independently
// refuses an unsafe start (ADR-0046/0059). While waiting it says what the
// person at the device must do, and only then; once the device is found
// MP §2.3 runs to its end and there is no stop button.

import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import { emitAchievementEvent } from "./achievementEvents";
import { loadPreferredGateway } from "./gatewayPreference";
import { useTranslate } from "./i18n";
import { collectDevices } from "./treeUtils";
import { useProgrammingConsent } from "./useProgrammingConsent";
import type { ProjectTree } from "./bindings/ProjectTree";

const POLL_INTERVAL_MS = 500;
const DEFAULT_WAIT_SECONDS = 120;

interface AddressProgrammingPanelProps {
  project: ProjectTree | null;
}

function active(status: api.AddressProgrammingStatus | undefined): boolean {
  return status?.state === "waiting" || status?.state === "programming";
}

export default function AddressProgrammingPanel({ project }: AddressProgrammingPanelProps) {
  const t = useTranslate();
  const consent = useProgrammingConsent();
  // Suggestions only: the address typed is the one programmed.
  const planned = useMemo(
    () =>
      project === null
        ? []
        : [...collectDevices(project).values()]
            .filter((device) => device.address !== null)
            .sort((a, b) => (a.address ?? "").localeCompare(b.address ?? "", undefined, { numeric: true })),
    [project],
  );
  const [address, setAddress] = useState("");
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const [waitSeconds, setWaitSeconds] = useState(String(DEFAULT_WAIT_SECONDS));
  const [starting, setStarting] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [status, setStatus] = useState<api.AddressProgrammingStatusResponse | null>(null);
  const [events, setEvents] = useState<api.AddressProgrammingEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [availability, setAvailability] = useState<api.AddressProgrammingAvailability | null>(null);
  const [availabilityError, setAvailabilityError] = useState<string | null>(null);
  const availabilityRequestRef = useRef(0);
  const sinceRef = useRef(0);
  const pollInFlightRef = useRef(false);
  const idRef = useRef<number | null>(null);

  const running = active(status?.status);
  const locked = starting || running;

  async function refreshAvailability() {
    const requestId = ++availabilityRequestRef.current;
    setAvailability(null);
    setAvailabilityError(null);
    try {
      const next = await api.addressProgrammingAvailability();
      if (requestId !== availabilityRequestRef.current) return;
      const valid = typeof next?.startAvailable === "boolean"
        && (next.reason === null || typeof next.reason === "string")
        && (!next.startAvailable || next.reason === null);
      if (!valid) {
        setAvailabilityError(t("addressProgramming.availabilityInvalid"));
        return;
      }
      setAvailability(next);
    } catch (reason) {
      if (requestId === availabilityRequestRef.current) setAvailabilityError(api.errorMessage(reason));
    }
  }

  useEffect(() => {
    void refreshAvailability();
    return () => { availabilityRequestRef.current += 1; };
  }, []);

  // ADR-0089: only `written: "yes"` — the device answers at the new
  // address — counts; reported once per programming run.
  const reportedRef = useRef<number | null>(null);
  function apply(next: api.AddressProgrammingStatusResponse) {
    if (idRef.current !== null && next.programmingId !== idRef.current) return;
    if (next.status.state === "finished" && next.status.written === "yes" && reportedRef.current !== next.programmingId) {
      reportedRef.current = next.programmingId;
      emitAchievementEvent({ type: "individualAddressVerified" });
    }
    idRef.current = next.programmingId;
    sinceRef.current = next.nextSince;
    setStatus(next);
    if (next.events.length > 0) setEvents((current) => [...current, ...next.events]);
  }

  async function poll() {
    if (pollInFlightRef.current) return;
    pollInFlightRef.current = true;
    try {
      apply(await api.pollAddressProgramming(sinceRef.current, idRef.current ?? undefined));
    } catch (reason) {
      // 409: a start is connecting right now; the next poll sees it.
      const code = api.errorStatus(reason);
      if (code !== 404 && code !== 409) setError(api.errorMessage(reason));
    } finally {
      pollInFlightRef.current = false;
    }
  }

  // Shows a programming that is already running (or finished) when the panel opens.
  useEffect(() => {
    void poll();
  }, []);

  useEffect(() => {
    if (!running) return;
    const interval = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [running, status?.programmingId]);

  async function start() {
    if (availability?.startAvailable !== true) return;
    const target = address.trim();
    setError(null);
    let phrase: api.AddressProgrammingPhrase;
    try {
      // Validates the address and names the phrase; sends nothing.
      phrase = await api.addressProgrammingPhrase(target);
    } catch (reason) {
      setError(api.errorMessage(reason));
      return;
    }
    const seconds = Number(waitSeconds);
    if (!Number.isInteger(seconds) || seconds < 1 || seconds > phrase.maxWaitSeconds) {
      setError(t("addressProgramming.waitOutOfRange", { max: phrase.maxWaitSeconds }));
      return;
    }
    if (!(await consent.request(t("addressProgramming.consentTarget", { address: phrase.address })))) return;
    setStarting(true);
    try {
      const { programmingId } = await api.startAddressProgramming(
        phrase.address,
        gateway.trim(),
        // The server's phrase for exactly the address the user confirmed.
        phrase.confirmationPhrase,
        seconds,
      );
      idRef.current = programmingId;
      sinceRef.current = 0;
      setEvents([]);
      setStatus(null);
      await poll();
    } catch (reason) {
      if (api.errorStatus(reason) === 412) {
        // The recovery gate can change after an earlier read. The server's
        // refusal is authoritative; do not invite a second consent attempt.
        setAvailability({ startAvailable: false, reason: api.errorMessage(reason) });
      } else {
        setError(api.errorMessage(reason));
      }
    } finally {
      setStarting(false);
    }
  }

  async function stop() {
    if (idRef.current === null) return;
    setStopping(true);
    try {
      await api.stopAddressProgramming(idRef.current);
      await poll();
    } catch (reason) {
      // 409: the device was found in the meantime; the procedure runs on.
      setError(api.errorMessage(reason));
    } finally {
      setStopping(false);
    }
  }

  const current = status?.status;
  const rounds = events.filter(
    (event): event is Extract<api.AddressProgrammingEvent, { kind: "round" }> => event.kind === "round",
  );

  return (
    <section className="address-programming-panel">
      <header>
        <p className="eyebrow">{t("addressProgramming.eyebrow")}</p>
        <h2>{t("addressProgramming.title")}</h2>
        <p>{t("addressProgramming.explainer")}</p>
      </header>

      {availability?.startAvailable !== true && (
        <div className="form-warning" role={availabilityError ? "alert" : "status"}>
          <p>{availabilityError
            ? t("addressProgramming.availabilityFailed")
            : availability
              ? t("addressProgramming.recoveryBlocked")
              : t("addressProgramming.availabilityChecking")}</p>
          {availability?.reason && <p>{availability.reason}</p>}
          {availabilityError && <p>{availabilityError}</p>}
          {(availability || availabilityError) && (
            <button type="button" onClick={() => void refreshAvailability()}>
              {t("addressProgramming.retryAvailability")}
            </button>
          )}
        </div>
      )}

      <form className="address-programming-config" onSubmit={(event) => event.preventDefault()}>
        <label>
          {t("addressProgramming.newAddress")}
          <input
            name="address"
            value={address}
            disabled={locked}
            list="address-programming-planned"
            onChange={(event) => setAddress(event.target.value)}
            placeholder="1.1.30"
          />
          <datalist id="address-programming-planned">
            {planned.map((device) => (
              <option key={device.id} value={device.address ?? ""}>
                {device.name}
              </option>
            ))}
          </datalist>
        </label>
        <label>
          {t("addressProgramming.gateway")}
          <input name="gateway" value={gateway} disabled={locked} onChange={(event) => setGateway(event.target.value)} placeholder="192.0.2.10:3671" />
        </label>
        <label>
          {t("addressProgramming.waitSeconds")}
          <input name="wait" type="number" min={1} value={waitSeconds} disabled={locked} onChange={(event) => setWaitSeconds(event.target.value)} />
        </label>
      </form>

      {!running && (
        <section className="address-programming-plan" aria-label={t("addressProgramming.planTitle")}>
          <h3>{t("addressProgramming.planTitle")}</h3>
          <ol>
            <li>{t("addressProgramming.planWait", { seconds: waitSeconds })}</li>
            <li>{t("addressProgramming.planStep1", { address: address.trim() || "…" })}</li>
            <li>{t("addressProgramming.planStep2")}</li>
            <li>{t("addressProgramming.planStep3", { address: address.trim() || "…" })}</li>
            <li>{t("addressProgramming.planStep4", { address: address.trim() || "…" })}</li>
          </ol>
          <button
            className="primary-action"
            disabled={locked || availability?.startAvailable !== true || address.trim() === "" || gateway.trim() === ""}
            onClick={() => void start()}
          >
            {t("addressProgramming.start", { address: address.trim() || "…" })}
          </button>
        </section>
      )}

      {error && <p className="form-error" role="alert">{error}</p>}

      {status && current && (
        <section className="address-programming-progress" aria-live="polite">
          <h3>{t("addressProgramming.progressTitle", { address: status.address })}</h3>
          {current.state === "waiting" && (
            <>
              <p className="address-programming-instruction" data-instruction={instruction(current.inProgrammingMode)}>
                {current.inProgrammingMode.length === 0
                  ? t("addressProgramming.pressButton")
                  : current.inProgrammingMode.length === 1
                    ? t("addressProgramming.foundOne", { device: current.inProgrammingMode[0] })
                    : t("addressProgramming.releaseAllButOne", { devices: current.inProgrammingMode.join(", ") })}
              </p>
              <p>{t("addressProgramming.waiting", { rounds: current.rounds, seconds: status.waitSeconds })}</p>
              <button disabled={stopping} onClick={() => void stop()}>
                {t("addressProgramming.stop")}
              </button>
            </>
          )}
          {current.state === "programming" && (
            <p className="form-warning" role="status">
              {t("addressProgramming.programming", { previous: current.previousAddress, address: status.address })}
            </p>
          )}
          {current.state === "finished" && (
            <p className="address-programming-written" data-written={current.written}>
              {current.written === "yes"
                ? t("addressProgramming.written.yes", { previous: current.previousAddress, address: status.address })
                : t("addressProgramming.written.noNeed", { address: status.address })}
            </p>
          )}
          {current.state === "stopped" && (
            <p className="address-programming-written" data-written="no">
              {t("addressProgramming.stopped", { rounds: current.rounds })}
            </p>
          )}
          {current.state === "failed" && (
            <>
              <p className={current.written === "unconfirmed" ? "form-warning" : "address-programming-written"} data-written={current.written}>
                {current.written === "unconfirmed"
                  ? t("addressProgramming.written.unconfirmed", { address: status.address, step: current.step ?? "—" })
                  : t("addressProgramming.written.no")}
              </p>
              <p className="form-error">{current.error}</p>
            </>
          )}
          {rounds.length > 0 && (
            <ul className="address-programming-rounds">
              {rounds.map((round) => (
                <li key={round.number}>
                  {t("addressProgramming.round", { number: round.number })}{" "}
                  {round.inProgrammingMode.length === 0 ? t("addressProgramming.nobody") : round.inProgrammingMode.join(", ")}
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
      {consent.dialog}
    </section>
  );
}

function instruction(devices: string[]): "press" | "wait" | "release" {
  if (devices.length === 0) return "press";
  return devices.length === 1 ? "wait" : "release";
}
