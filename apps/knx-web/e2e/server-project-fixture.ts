/** Answers GET /api/project like a server with no open project, for whole-app fixtures. */
// ADR-0127: every editor mount reads `GET /api/project` once to resume what
// the server still holds. The real server answers `400 no project open`
// until a project exists (`domain::current_project_tree`), and the app then
// stays on the welcome screen silently. A fixture that answered with a
// tree before the test created one would resume it at once; one that
// answered 404 or aborted would raise an error toast. Fixtures use this
// one copy of the real answer instead.
import type { Route } from "@playwright/test";

/** The exact status and body the server sends while nothing is open. */
export const NO_PROJECT_OPEN = { status: 400, contentType: "application/json", body: '{"error":"no project open"}' };

/** Fulfils `route` with the real server's answer while no project is open. */
export function fulfillNoProjectOpen(route: Route): Promise<void> {
  return route.fulfill(NO_PROJECT_OPEN);
}
