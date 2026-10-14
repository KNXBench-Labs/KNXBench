/** About links the community Hall of Fame only when the user chooses to open it. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { HALL_OF_FAME_BOARD_URL } from "./hallOfFame";
import { useTranslate } from "./i18n";

export default function HallOfFameSection() {
  const t = useTranslate();
  return <p className="hall-of-fame" data-hall-of-fame="">
    <a href={HALL_OF_FAME_BOARD_URL} target="_blank" rel="noopener noreferrer" data-hall-of-fame-board="">
      {t("about.hallOfFame.board")}</a>
  </p>;
}
