/** The Hall of Fame link always opens the public document in this project's repository. */
import { expect, it } from "vitest";
import { HALL_OF_FAME_BOARD_URL } from "./hallOfFame";

it("links the canonical GitHub Hall of Fame document", () => {
  expect(HALL_OF_FAME_BOARD_URL).toBe("https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/community/HALL_OF_FAME.md");
});
