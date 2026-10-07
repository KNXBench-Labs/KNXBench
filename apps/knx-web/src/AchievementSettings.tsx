/** The achievements settings section: the on/off switch and a confirmed, non-destructive reset. */
// ADR-0089. The reset asks first, inline rather than in a second modal on
// top of Settings, and says afterwards where the previous record went:
// the server moves `achievements.json` aside rather than deleting it.
import { useState } from "react";
import type { AchievementTracker } from "./achievementTracker";
import { useAchievementsEnabled } from "./achievementPreference";
import { useTranslate } from "./i18n";

type ResetOutcome = { kind: "done"; file: string } | { kind: "nothing" } | { kind: "failed" };

export default function AchievementSettings(props: { tracker: AchievementTracker }) {
  const t = useTranslate();
  const [enabled, setEnabled] = useAchievementsEnabled();
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<ResetOutcome>();

  async function reset() {
    setBusy(true);
    try {
      const reply = await props.tracker.reset();
      setOutcome(reply.movedTo ? { kind: "done", file: reply.movedTo } : { kind: "nothing" });
    } catch {
      setOutcome({ kind: "failed" });
    } finally {
      setBusy(false);
      setConfirming(false);
    }
  }

  return (
    <section className="settings-section settings-section-achievements">
      <h3>{t("settings.section.achievements")}</h3>
      <label className="settings-field settings-field-checkbox">
        <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
        <span className="settings-field-label">{t("settings.achievementsEnabled")}</span>
      </label>
      <span className="settings-field-hint">{t("settings.achievementsEnabledHint")}</span>
      {confirming ? (
        <div className="settings-field achievements-reset-confirm" role="group">
          <span className="settings-field-hint">{t("settings.achievementsResetConfirm")}</span>
          <div className="achievements-reset-actions">
            <button type="button" onClick={() => setConfirming(false)} disabled={busy}>
              {t("settings.achievementsResetCancel")}
            </button>
            <button type="button" onClick={() => void reset()} disabled={busy}>
              {t("settings.achievementsResetConfirmButton")}
            </button>
          </div>
        </div>
      ) : (
        <button type="button" onClick={() => { setOutcome(undefined); setConfirming(true); }}>
          {t("settings.achievementsReset")}
        </button>
      )}
      {outcome && (
        <p className="settings-diagnostic" role="status">
          {outcome.kind === "done" ? t("settings.achievementsResetDone", { file: outcome.file })
            : outcome.kind === "nothing" ? t("settings.achievementsResetNothing")
            : t("settings.achievementsResetFailed")}
        </p>
      )}
    </section>
  );
}
