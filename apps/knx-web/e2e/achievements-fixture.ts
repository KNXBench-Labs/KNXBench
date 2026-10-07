/** Answers the achievements routes for browser fixtures that mount the whole application. */
// ADR-0089: the workbench reads `GET /api/achievements` on start and may
// report unlocks with `POST /api/achievements/record`. Fixtures that are
// not about achievements answer with an empty record, so the tracker
// behaves as on a fresh installation and the spec's own request ledger
// stays strict about everything else.

const EMPTY_RECORD = { schemaVersion: 1, unlocked: {}, progress: {} };

/** The JSON body for an achievements route, or `undefined` for any other request. */
export function achievementsFixtureAnswer(method: string, path: string): unknown {
  if (path === "/api/achievements" && method === "GET") return { ...EMPTY_RECORD, status: "absent" };
  if (path === "/api/achievements/record" && method === "POST") return { ...EMPTY_RECORD, status: "ok" };
  if (path === "/api/achievements/reset" && method === "POST") return { ...EMPTY_RECORD, status: "absent" };
  return undefined;
}
