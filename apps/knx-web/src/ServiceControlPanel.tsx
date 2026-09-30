/** Manual service-control Debug action that never contacts a bus on mount. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { loadPreferredGateway } from "./gatewayPreference";
import { splitGatewayEndpoint, validateGatewayFields } from "./gatewayEndpoint";
import { useTranslate } from "./i18n";

const CONTACTABLE_ADDRESS = /^(?:[0-9]|1[0-5])\.(?:[0-9]|1[0-5])\.(?:[1-9]|[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5])$/;
const HEX_WORD = /^[0-9A-Fa-f]{4}$/;

interface Review {
  address: string;
  gateway: string;
  before: api.ServiceControlReading;
  enable: boolean;
}

function validReading(read: api.ServiceControlReading | null | undefined, address: string): boolean {
  return read != null && read.address === address &&
    typeof read.raw === "string" && typeof read.mask === "string" &&
    HEX_WORD.test(read.raw) && HEX_WORD.test(read.mask) &&
    read.mask.toUpperCase() !== "0021" && typeof read.individualAddressWriteEnabled === "boolean" &&
    ((parseInt(read.raw, 16) & 4) !== 0) === read.individualAddressWriteEnabled;
}

function validResult(result: api.ServiceControlWriteResponse | null | undefined, review: Review): boolean {
  if (result == null || !validReading(result.before, review.address) ||
      result.before.raw.toUpperCase() !== review.before.raw.toUpperCase() ||
      result.before.mask.toUpperCase() !== review.before.mask.toUpperCase() ||
      typeof result.after !== "string" || !HEX_WORD.test(result.after) ||
      typeof result.written !== "boolean" ||
      typeof result.individualAddressWriteEnabled !== "boolean") return false;
  const before = parseInt(result.before.raw, 16);
  const after = parseInt(result.after, 16);
  return ((after & 4) !== 0) === review.enable &&
    result.individualAddressWriteEnabled === review.enable &&
    (before & ~4) === (after & ~4) &&
    (result.written ? before !== after : before === after);
}

export default function ServiceControlPanel({ projectOpen, projectRevision }: {
  projectOpen: boolean;
  projectRevision?: unknown;
}) {
  const t = useTranslate();
  const [address, setAddress] = useState("");
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const [reading, setReading] = useState<api.ServiceControlReading | null>(null);
  const [review, setReview] = useState<Review | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState<"reading" | "writing" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const version = useRef(0);
  const inFlight = useRef(false);
  const mounted = useRef(true);

  function invalidate() {
    ++version.current;
    setReading(null);
    setReview(null);
    setConfirmation("");
    setResult(null);
    setError(null);
    if (!inFlight.current) setBusy(null);
  }
  useEffect(() => {
    mounted.current = true;
    invalidate();
    return () => { mounted.current = false; ++version.current; };
  }, [projectOpen, projectRevision]);

  async function read() {
    if (!projectOpen || busy !== null || inFlight.current) return;
    invalidate();
    if (!CONTACTABLE_ADDRESS.test(address)) {
      setError(t("serviceControl.invalidAddress"));
      return;
    }
    const checked = validateGatewayFields(splitGatewayEndpoint(gateway));
    if (!checked.ok) {
      setError(t(checked.reason === "hostRequired" ? "busMonitor.connectNeedsGateway" :
        checked.reason === "invalidPort" ? "busMonitor.invalidPort" : "busMonitor.invalidHost"));
      return;
    }
    const target = address;
    const active = ++version.current;
    inFlight.current = true;
    setBusy("reading");
    try {
      const next = await api.readServiceControl(target, checked.endpoint);
      if (version.current !== active) return;
      if (!validReading(next, target)) setError(t("serviceControl.unexpectedRead"));
      else setReading(next);
    } catch (reason) {
      if (version.current === active) setError(api.errorMessage(reason));
    } finally {
      inFlight.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  function beginReview() {
    if (!projectOpen || busy !== null || inFlight.current || reading === null || !validReading(reading, address)) return;
    const checked = validateGatewayFields(splitGatewayEndpoint(gateway));
    if (!checked.ok) return;
    setReview({ address, gateway: checked.endpoint, before: reading, enable: !reading.individualAddressWriteEnabled });
    setConfirmation("");
    setResult(null);
    setError(null);
  }

  async function write() {
    if (!projectOpen || busy !== null || inFlight.current || review === null ||
        confirmation !== `I confirm individual-address write enable to ${review.address}`) return;
    const selected = review;
    const phrase = confirmation;
    const active = ++version.current;
    inFlight.current = true;
    setReview(null);
    setConfirmation("");
    setResult(null);
    setError(null);
    setBusy("writing");
    try {
      const next = await api.writeServiceControl(selected.address, selected.gateway, selected.enable, phrase);
      if (version.current !== active) return;
      setReading(null); // a failed or contradictory result must be read again
      if (!validResult(next, selected)) setError(t("serviceControl.unexpectedWrite"));
      else if (next.written && (typeof next.backupPath !== "string" || !next.backupPath.trim())) {
        setError(t("serviceControl.backupMissing"));
      } else if (!next.written && next.backupPath !== null) setError(t("serviceControl.unexpectedWrite"));
      else setResult(next.written
        ? t("serviceControl.verified", { path: next.backupPath! })
        : t("serviceControl.noChange"));
    } catch (reason) {
      if (version.current === active) {
        setReading(null);
        setError(api.errorMessage(reason));
      }
    } finally {
      inFlight.current = false;
      if (mounted.current) setBusy(null);
    }
  }

  const phrase = review === null ? "" : `I confirm individual-address write enable to ${review.address}`;
  return <section className="service-control-panel">
    <header>
      <p className="eyebrow">{t("serviceControl.tab")}</p>
      <h2>{t("serviceControl.title")}</h2>
      <p>{t("serviceControl.intro")}</p>
    </header>
    <p className="service-control-warning">{t("serviceControl.warning")}</p>
    {!projectOpen && <p>{t("serviceControl.projectRequired")}</p>}
    <div className="service-control-fields">
      <label>{t("serviceControl.address")}
        <input name="address" value={address} disabled={!projectOpen || busy !== null}
          onChange={(event) => { invalidate(); setAddress(event.target.value); }} placeholder="1.1.67" />
      </label>
      <label>{t("serviceControl.gateway")}
        <input name="gateway" value={gateway} disabled={!projectOpen || busy !== null}
          onChange={(event) => { invalidate(); setGateway(event.target.value); }} placeholder="192.0.2.10:3671" />
      </label>
    </div>
    <button type="button" className="service-control-read" disabled={!projectOpen || busy !== null}
      onClick={() => void read()}>{busy === "reading" ? t("serviceControl.reading") : t("serviceControl.read")}</button>
    {reading && <div className="service-control-observation">
      <p>{t("serviceControl.current", { raw: reading.raw, mask: reading.mask,
        enabled: t(reading.individualAddressWriteEnabled ? "serviceControl.enabled" : "serviceControl.disabled") })}</p>
      <p>{t(reading.individualAddressWriteEnabled ? "serviceControl.disable" : "serviceControl.enable")}</p>
      {!review && <button type="button" className="service-control-review" onClick={beginReview}>{t("serviceControl.review")}</button>}
    </div>}
    {review && <div className="service-control-review" role="group" aria-label={t("serviceControl.review")}>
      <p>{t("serviceControl.reviewHint", { address: review.address, gateway: review.gateway })}</p>
      <label>{t("serviceControl.phrase", { phrase })}
        <input name="confirmation" value={confirmation} autoComplete="off" spellCheck={false}
          onChange={(event) => setConfirmation(event.target.value)} />
      </label>
      <div className="service-control-actions">
        <button className="service-control-write" type="button" disabled={confirmation !== phrase || busy !== null}
          onClick={() => void write()}>{t("serviceControl.write")}</button>
        <button type="button" onClick={() => { setReview(null); setConfirmation(""); }}>{t("serviceControl.cancel")}</button>
      </div>
    </div>}
    {busy === "writing" && <p role="status">{t("serviceControl.writing")}</p>}
    {error && <p className="form-error" role="alert">{error}</p>}
    {result && <p role="status" className="service-control-result">{result}</p>}
  </section>;
}
