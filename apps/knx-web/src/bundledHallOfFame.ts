/** The Hall of Fame snapshot this build ships with: read at build time, never fetched. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import text from "../../../docs/community/hall-of-fame.json?raw";
import { parseHallOfFame } from "./hallOfFame";

// A release carries the board as it was when it was built; About says so and
// links the live one. An unreadable file becomes a sentence, not a crash, and
// `hallOfFame.test.ts` fails loudly if the bundled file stops being admitted.
export const BUNDLED_HALL_OF_FAME = parseHallOfFame(text);
