/** Renders the single lossless editor for protected line-scan individual addresses. */
import { useState } from "react";
import { useTranslate } from "./i18n";
import {
  saveLineScanExclusions,
  useLineScanExclusions,
  validateIndividualAddress,
} from "./lineScanExclusions";

type RemovalConfirmation = {
  index: number;
  snapshot: readonly string[];
};

function confirmsCurrentOccurrence(
  confirmation: RemovalConfirmation | null,
  index: number,
  exclusions: readonly string[],
): boolean {
  return confirmation?.index === index
    && confirmation.snapshot.length === exclusions.length
    && confirmation.snapshot.every((value, current) => value === exclusions[current]);
}

export default function LineScanExclusionsEditor({
  disabled,
  values,
}: {
  disabled: boolean;
  values?: readonly string[];
}) {
  const t = useTranslate();
  const storedExclusions = useLineScanExclusions();
  const exclusions = values ?? storedExclusions;
  const [candidate, setCandidate] = useState("");
  const [confirmRemoval, setConfirmRemoval] = useState<RemovalConfirmation | null>(null);
  const [addError, setAddError] = useState<string | null>(null);

  function invalidAt(value: string, index: number): boolean {
    return !validateIndividualAddress(value) || exclusions.indexOf(value) !== index;
  }

  function add() {
    if (!validateIndividualAddress(candidate)) {
      setAddError(t("lineScan.invalidExclusion"));
      return;
    }
    if (exclusions.includes(candidate)) {
      setAddError(t("lineScan.duplicateExclusion"));
      return;
    }
    saveLineScanExclusions([...exclusions, candidate]);
    setCandidate("");
    setAddError(null);
  }

  function remove(index: number) {
    if (!confirmsCurrentOccurrence(confirmRemoval, index, exclusions)) {
      setConfirmRemoval({ index, snapshot: [...exclusions] });
      return;
    }
    saveLineScanExclusions(exclusions.filter((_, current) => current !== index));
    setConfirmRemoval(null);
  }

  return (
    <aside className="line-scan-exclusions">
      <h3>{t("lineScan.exclusions")}</h3>
      {exclusions.length === 0 ? <p>{t("lineScan.noExclusions")}</p> : (
        <ul>{exclusions.map((address, index) => {
          const invalid = invalidAt(address, index);
          return <li key={`${index}:${address}`} data-invalid={invalid ? "true" : undefined}>
            <code>{address}</code>
            <span>{invalid ? t("lineScan.invalidLegacyExclusion") : t("lineScan.protected")}</span>
            <button type="button" disabled={disabled} onClick={() => remove(index)}>
              {confirmsCurrentOccurrence(confirmRemoval, index, exclusions)
                ? t("lineScan.confirmRemoval")
                : t("lineScan.remove")}
            </button>
          </li>;
        })}</ul>
      )}
      <div className="line-scan-add-exclusion">
        <input
          disabled={disabled}
          aria-label={t("lineScan.exclusionAddress")}
          value={candidate}
          onChange={(event) => { setCandidate(event.target.value); setAddError(null); }}
          placeholder="2.3.42"
        />
        <button type="button" disabled={disabled} onClick={add}>{t("lineScan.addExclusion")}</button>
      </div>
      {addError && <p className="form-error" role="alert">{addError}</p>}
    </aside>
  );
}
