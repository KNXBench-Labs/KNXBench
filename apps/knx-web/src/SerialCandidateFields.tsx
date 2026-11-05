/** Offer explicit, unverified server-local serial candidates without changing connection state. */
import { useEffect, useRef, useState } from "react";
import { listSerialCandidates } from "./api";
import { useTranslate } from "./i18n";
import type { SerialCandidates } from "./serialCandidates";

type Listing = SerialCandidates | { status: "idle" | "loading" | "failed" | "legacy" };
export default function SerialCandidateFields({ disabled, onSelect }: {
  disabled: boolean; onSelect: (path: string) => void;
}) {
  const t = useTranslate();
  const [listing, setListing] = useState<Listing>({ status: "idle" });
  const generation = useRef(0);
  const inFlight = useRef(false);
  useEffect(() => () => { generation.current += 1; }, []);
  async function list() {
    // Keep the focused control mounted; aria-disabled needs a synchronous effect guard.
    if (disabled || inFlight.current) return;
    inFlight.current = true;
    const own = ++generation.current;
    setListing({ status: "loading" });
    try {
      const response = await listSerialCandidates();
      if (own === generation.current) setListing(response);
    } catch (error) {
      if (own !== generation.current) return;
      const legacy = error !== null && typeof error === "object" && "status" in error && error.status === 404;
      setListing({ status: legacy ? "legacy" : "failed" });
    } finally {
      if (own === generation.current) inFlight.current = false;
    }
  }
  return <div className="serial-candidate-fields">
    <button type="button" disabled={disabled} aria-disabled={disabled || listing.status === "loading"}
      onClick={() => void list()}>
      {t("serialCandidates.list")}
    </button>
    <p role="note">{t("serialCandidates.warning")}</p>
    <div role="status" aria-live="polite" aria-busy={listing.status === "loading"}>
      {listing.status === "loading" && <p>{t("serialCandidates.loading")}</p>}
      {listing.status === "failed" && <p>{t("serialCandidates.failed")}</p>}
      {listing.status === "legacy" && <p>{t("serialCandidates.legacy")}</p>}
      {listing.status === "unsupported" && <p>{t("serialCandidates.unsupported")}</p>}
      {listing.status === "unavailable" && <p>{t(`serialCandidates.${listing.reason}`)}</p>}
      {listing.status === "available" && <>
        <p>{t(listing.candidates.length === 0 ? "serialCandidates.empty" : "serialCandidates.available", { count: listing.candidates.length })}</p>
        {listing.truncated && <p>{t("serialCandidates.truncated")}</p>}
        {listing.omittedEntries > 0 && <p>{t("serialCandidates.omitted", { count: listing.omittedEntries })}</p>}
      </>}
    </div>
    {listing.status === "available" && listing.candidates.length > 0 && <ul aria-label={t("serialCandidates.results")}>
      {listing.candidates.map(row => <li key={row.path}>
        <button type="button" disabled={disabled} onClick={() => onSelect(row.path)}>{t("serialCandidates.use", { path: row.path })}</button>
      </li>)}
    </ul>}
  </div>;
}
