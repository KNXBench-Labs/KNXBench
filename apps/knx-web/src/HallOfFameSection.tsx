/** About's community Hall of Fame: every bundled participant and the way to the live board. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { BUNDLED_HALL_OF_FAME } from "./bundledHallOfFame";
import { HALL_OF_FAME_BOARD_URL, type HallOfFameEntry } from "./hallOfFame";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";

// Titles and medals stay the generator's English words in every UI language
// (docs/contracts/community-recognition.md); handles are React text, so markup
// in one is shown, never run, and only a well-formed handle becomes a link.
function Entry(props: { entry: HallOfFameEntry }) {
  const t = useTranslate();
  const { entry } = props;
  return <li className="hall-of-fame-entry" value={entry.position} data-hall-of-fame-entry={entry.login}>
    <span className="hall-of-fame-who">
      {entry.profileUrl
        ? <a href={entry.profileUrl} target="_blank" rel="noopener noreferrer">{entry.login}</a>
        : <span>{entry.login}</span>}
      <span className="hall-of-fame-xp">{t("about.hallOfFame.xp", { xp: entry.xp })}</span>
    </span>
    <span className="hall-of-fame-title">
      {entry.title}
      {entry.medals.length > 0 && <> · <span aria-hidden="true">🏅</span> {entry.medals.join(", ")}</>}
    </span>
    <span className="hall-of-fame-counters">
      {t("about.hallOfFame.counters", { issues: entry.issues, replies: entry.replies, resolutions: entry.resolutions })}
    </span>
  </li>;
}

export default function HallOfFameSection() {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const result = BUNDLED_HALL_OF_FAME;
  // A disclosure, so About keeps its compact size (dialogs-overview.e2e.ts pins it);
  // opened, it shows every participant.
  const count = result.ok ? result.snapshot.entries.length : null;
  return <details className="hall-of-fame" data-hall-of-fame="">
    <summary id="hall-of-fame-title">
      {count === null ? t("about.hallOfFame.title") : t("about.hallOfFame.titleCount", { count })}</summary>
    <p>{t("about.hallOfFame.intro")}</p>
    {!result.ok && <p role="status">{t("about.hallOfFame.unavailable")}</p>}
    {result.ok && <>
      <p className="hall-of-fame-asof">
        {t("about.hallOfFame.asOf")}{" "}
        <time dateTime={result.snapshot.generatedAt}>{new Date(result.snapshot.generatedAt).toLocaleString(language)}</time>.{" "}
        {t("about.hallOfFame.notLive")}
      </p>
      {result.snapshot.entries.length === 0
        ? <p>{t("about.hallOfFame.empty")}</p>
        : <ol className="hall-of-fame-list">{result.snapshot.entries.map((entry) => <Entry key={entry.login} entry={entry} />)}</ol>}
    </>}
    <a href={HALL_OF_FAME_BOARD_URL} target="_blank" rel="noopener noreferrer" data-hall-of-fame-board="">
      {t("about.hallOfFame.board")}</a>
  </details>;
}
