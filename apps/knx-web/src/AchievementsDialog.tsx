/** The achievements overview: every achievement with its tier, state, date or progress. */
// ADR-0089. Unlocked achievements come first (catalogue order), then the
// locked ones. A locked hidden achievement shows neither its title nor
// its description — that is the whole point of hiding it — but it does
// show that it exists and its tier, so the count in the summary adds up.
import { ACHIEVEMENTS, achievementGoal, type AchievementDefinition } from "./achievementCatalog";
import type { TrackerSnapshot } from "./achievementTracker";
import AchievementBadge from "./AchievementBadge";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";

const STATUS_NOTE = {
  loading: "achievements.loading",
  unavailable: "achievements.unavailable",
  readOnly: "achievements.readOnly",
} as const;

export default function AchievementsDialog(props: {
  snapshot: TrackerSnapshot;
  enabled: boolean;
  onClose: () => void;
  catalog?: readonly AchievementDefinition[];
}) {
  const { snapshot, enabled, onClose } = props;
  const catalog = props.catalog ?? ACHIEVEMENTS;
  const t = useTranslate();
  const [language] = useUiLanguage();
  const { unlocked, progress } = snapshot.record;
  const isUnlocked = (a: AchievementDefinition) => a.id in unlocked;
  const ordered = [...catalog.filter(isUnlocked), ...catalog.filter((a) => !isUnlocked(a))];
  const unlockedCount = catalog.filter(isUnlocked).length;
  const statusNote = snapshot.status === "ready" ? undefined : STATUS_NOTE[snapshot.status];

  function formatDate(iso: string): string {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) return iso;
    return new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(date);
  }

  return (
    <Overlay labelledBy="achievements-title" className="achievements-dialog" onClose={onClose}>
      <header className="achievements-header">
        <h2 id="achievements-title">{t("achievements.title")}</h2>
        <span className="achievements-summary">
          {t("achievements.summary", { unlocked: unlockedCount, total: catalog.length })}
        </span>
      </header>
      {!enabled && <p className="achievements-note" role="status">{t("achievements.disabled")}</p>}
      {statusNote && <p className="achievements-note" role="status">{t(statusNote)}</p>}
      <ul className="achievements-list">
        {ordered.map((achievement) => {
          const done = isUnlocked(achievement);
          const secret = achievement.hidden && !done;
          const goal = achievementGoal(achievement);
          const current = Math.min(progress[achievement.id] ?? 0, goal ?? 0);
          return (
            <li
              key={achievement.id}
              data-achievement={achievement.id}
              className={done ? "achievement-item achievement-item--unlocked" : "achievement-item"}
            >
              <AchievementBadge tier={achievement.tier} glyph={secret ? "star" : achievement.glyph} locked={!done} />
              <div className="achievement-item-text">
                <span className="achievement-item-title">
                  {secret ? t("achievements.hiddenTitle") : t(achievement.titleKey)}
                </span>
                <span className="achievement-item-description">
                  {secret ? t("achievements.hiddenDescription") : t(achievement.descriptionKey)}
                </span>
                {!done && goal !== undefined && (
                  <span className="achievement-item-progress">
                    <progress value={current} max={goal} />
                    <span>{t("achievements.progress", { current, goal })}</span>
                  </span>
                )}
              </div>
              <div className="achievement-item-meta">
                <span className={`achievement-tier achievement-tier--${achievement.tier}`}>
                  {t(`achievements.tier.${achievement.tier}`)}
                </span>
                <span className="achievement-item-state">
                  {done
                    ? t("achievements.unlockedOn", { date: formatDate(unlocked[achievement.id]) })
                    : goal === undefined ? t("achievements.locked") : null}
                </span>
              </div>
            </li>
          );
        })}
      </ul>
      <footer className="achievements-footer">
        <button type="button" onClick={onClose}>{t("achievements.close")}</button>
      </footer>
    </Overlay>
  );
}
