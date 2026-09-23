/** fetch()-based HTTP client mirroring the Tauri invoke() surface the UI components call. */
//! fetch()-based replacement for @tauri-apps/api/core's invoke() — same
//! function names and argument shapes the components already called, so
//! swapping the import at each call site is the only change there.
import type { ProjectTree } from "./bindings/ProjectTree";
import type { SettingsDiagnostic } from "./settingsStore";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import { notifySessionExpired } from "./session";

/**
 * The three endpoints a login screen talks to (ADR-0026). Their own 401 is
 * an *answer* — "that password is wrong" — not a session that ended, so a
 * refusal from one of these must never be published as an expiry: doing so
 * would send `AuthGate` back to the login screen it is already showing, and
 * on the desktop shell would conjure one out of nothing.
 */
const AUTH_PATH_PREFIX = "/api/auth/";

/**
 * The one place the frontend learns that the server wants a session.
 *
 * Most `/api/` calls funnel through `request()`, which calls this itself.
 * The two that cannot — `installProductPackage`, which needs a streaming
 * body, and `FsPicker.tsx`'s two helpers, which need `FormData` and a raw
 * `Response` — call it by hand, which is why it is exported. Every route
 * they touch sits behind the same `route_layer` as the rest (ADR-0026), so
 * a `fetch` that skips this helper is a 401 that reaches nobody and a modal
 * the user cannot get out of.
 */
export function noteRefusal(path: string, status: number): void {
  if (status === 401 && !path.startsWith(AUTH_PATH_PREFIX)) {
    notifySessionExpired();
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    noteRefusal(path, response.status);
    throw requestError(response.status, body?.error ?? `${response.status} ${response.statusText}`);
  }
  if (response.headers.get("content-length") === "0") {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

// A plain `Error` with the HTTP status tacked on — not a subclass, so a
// caller can duck-type `(e as { status?: number }).status` without needing
// an `instanceof` check against a class this module would then have to keep
// exported through every `vi.mock("./api", ...)` factory in the test suite.
// `BusMonitorPanel.tsx`'s mount-time reattach uses this to tell "no session
// exists yet" (`404`) apart from every other failure, which it does not
// silently swallow the same way.
function requestError(status: number, message: string): Error {
  const error = new Error(message) as Error & { status: number };
  error.status = status;
  return error;
}

/**
 * `clientToken` is the opaque per-load id `App.tsx`'s `runLoad` generates
 * with `crypto.randomUUID()` (ADR-0023 fix round 3, F9). The server stores
 * it on the operation and echoes it in every `LoadProgressSnapshot`;
 * `loadProgress.ts`'s `ownsOperation` is exact equality against it, not a
 * fact this module derives.
 */
export function importProject(path: string, clientToken: string): Promise<ProjectTree> {
  return request("/api/project/import", {
    method: "POST",
    body: JSON.stringify({ path, clientToken }),
  });
}

export function openProject(path: string, clientToken: string): Promise<ProjectTree> {
  return request("/api/project/open", {
    method: "POST",
    body: JSON.stringify({ path, clientToken }),
  });
}

export function currentProject(): Promise<ProjectTree & { has_store_path: boolean }> {
  return request("/api/project");
}

/**
 * What the server is doing inside the one `importProject`/`openProject`
 * call that is still in flight — `GET /api/project/load-progress`, the
 * whole of ADR-0023's transport.
 *
 * `completed`/`total` are both `null` for every phase that has no real
 * count to report, which is most of them; the two that do are counting
 * things they have already finished, never predicting. Nothing here is
 * derived from elapsed time, and the snapshot deliberately carries no
 * timestamp for a caller to be tempted by.
 */
export interface LoadProgressSnapshot {
  /** Monotonic per server run, never reused. Not how a poller tells its
   * own load apart from anyone else's — see `clientToken` for that; this
   * is purely a server-side sequence number. */
  operationId: number;
  kind: "import" | "open";
  /** The file name, never the full path. */
  source: string;
  /** A wire phase name (`loadProgress.ts`'s `LOAD_PHASES`). */
  phase: string;
  completed: number | null;
  total: number | null;
  status: "running" | "succeeded" | "failed";
  /** Set only when `status` is `"failed"` — the same message the POST
   * rejected with, kept for a client that lost that response. */
  error: string | null;
  /** The `clientToken` this operation was started with, or `null` when
   * none was sent. `loadProgress.ts`'s `ownsOperation` is exact equality
   * between this and the token `runLoad` generated for its own load
   * (ADR-0023 fix round 3, F9) — the entire ownership test. */
  clientToken: string | null;
}

/** `null` when this server run has never loaded anything. */
export function loadProgress(): Promise<LoadProgressSnapshot | null> {
  return request("/api/project/load-progress");
}

/**
 * The three spellings `POST /api/project/new` accepts for
 * `groupAddressStyle` — the same tokens ETS writes into a `.knxproj` and
 * `knx-store` persists, so nothing here needs translating on the way to
 * the server (see `routes.rs`'s `parse_group_address_style`). What the
 * *user* reads is a catalogue entry per member; these are wire values,
 * never labels.
 */
export type GroupAddressStyle = "Free" | "TwoLevel" | "ThreeLevel";

export interface NewProjectOptions {
  name: string;
  installationName: string;
  /** BCP-47 tag the project's own texts are stored under
   * (`knx_core::Language`), unrelated to the UI chrome language. */
  language: string;
  /** Effectively permanent once group addresses exist — nothing in the
   * domain restyles a project afterwards — which is why it is asked for
   * at creation rather than left to a default. */
  groupAddressStyle: GroupAddressStyle;
  /**
   * Throws away the open project's unsaved edits. Only ever `true`
   * because a human read what was at stake and said so: the server
   * refuses with `409` by default precisely so this cannot happen by
   * omission, and re-sending with this flag is the one way past it.
   */
  discardChanges?: boolean;
}

/**
 * Creates an empty project server-side, replacing whatever is open. The
 * only path to a project that never came from a file — an ETS import or
 * a `.knxdb` open are the other two — and therefore the entry point for
 * installing a device from the product catalogue with no `.knxproj` in
 * sight.
 *
 * Rejects with a `409`-carrying error when the open project has unsaved
 * edits and `discardChanges` was not set; `isUnsavedChangesConflict`
 * below is how a caller tells that apart from a real failure.
 */
export function newProject(options: NewProjectOptions): Promise<ProjectTree> {
  return request("/api/project/new", {
    method: "POST",
    body: JSON.stringify({
      name: options.name,
      installationName: options.installationName,
      language: options.language,
      groupAddressStyle: options.groupAddressStyle,
      // Always explicit, never omitted: `false` on the wire says the same
      // thing as an absent field to the server, and saying it out loud
      // makes "did this request ask to discard anything?" answerable by
      // looking at the body alone.
      discardChanges: options.discardChanges === true,
    }),
  });
}

/** Whether a `newProject` rejection is the server's "the open project has
 * unsaved changes" refusal (`409`) rather than a failure — the one error
 * a caller must turn into a question for the user instead of a toast. */
export function isUnsavedChangesConflict(e: unknown): boolean {
  return errorStatus(e) === 409;
}

export function saveProject(): Promise<void> {
  return request("/api/project/save", { method: "POST" });
}

export function saveProjectAs(path: string): Promise<void> {
  return request("/api/project/save-as", { method: "POST", body: JSON.stringify({ path }) });
}

// `language` picks which stored translation row `com_objects[].name`/
// `.description` come back as (T33) — same convention as
// `deviceParameters` below, via the same `languageQuery` helper, so the
// two device-detail-adjacent endpoints don't grow separate opinions about
// how an optional language becomes a query string.
export function deviceDetail(deviceId: number, language?: string | null): Promise<DeviceDetail> {
  return request(`/api/device/${deviceId}${languageQuery(language)}`);
}

export function setIndividualAddress(deviceId: number, address: string | null): Promise<ProjectTree> {
  return request("/api/individual-address", {
    method: "POST",
    body: JSON.stringify({ deviceId, address }),
  });
}

export function setComObjectDpt(comObjectId: number, dpt: string | null): Promise<ProjectTree> {
  return request("/api/com-object-dpt", {
    method: "POST",
    body: JSON.stringify({ comObjectId, dpt }),
  });
}

export function setDeviceDescription(
  deviceId: number,
  description: string | null,
): Promise<ProjectTree> {
  return request("/api/device-description", {
    method: "POST",
    body: JSON.stringify({ deviceId, description }),
  });
}

export function setComObjectDescription(
  comObjectId: number,
  description: string | null,
): Promise<ProjectTree> {
  return request("/api/com-object-description", {
    method: "POST",
    body: JSON.stringify({ comObjectId, description }),
  });
}

export type ComFlagName =
  | "Read"
  | "Write"
  | "Transmit"
  | "Update"
  | "Communication"
  | "ReadOnInit";

export function setComObjectFlag(
  comObjectId: number,
  flag: ComFlagName,
  value: boolean,
): Promise<ProjectTree> {
  return request("/api/com-object-flag", {
    method: "POST",
    body: JSON.stringify({ comObjectId, flag, value }),
  });
}

export function createGroupAddress(
  name: string,
  address: string,
  rangeId?: number,
): Promise<ProjectTree> {
  return request("/api/group-addresses", {
    method: "POST",
    body: JSON.stringify({ name, address, rangeId }),
  });
}

export function deleteGroupAddress(id: number): Promise<ProjectTree> {
  return request(`/api/group-addresses/${id}`, { method: "DELETE" });
}

export function createGroupRange(
  name: string,
  start: string,
  end: string,
  parentId?: number,
): Promise<ProjectTree> {
  return request("/api/group-ranges", {
    method: "POST",
    body: JSON.stringify({ name, start, end, parentId }),
  });
}

export function deleteGroupRange(id: number): Promise<ProjectTree> {
  return request(`/api/group-ranges/${id}`, { method: "DELETE" });
}

export function renameGroupRange(id: number, name: string): Promise<ProjectTree> {
  return request(`/api/group-ranges/${id}`, { method: "PATCH", body: JSON.stringify({ name }) });
}

export function createBuildingPart(
  name: string,
  kind: string,
  parentId?: number,
): Promise<ProjectTree> {
  return request("/api/building-parts", {
    method: "POST",
    body: JSON.stringify({ name, kind, parentId }),
  });
}

export function deleteBuildingPart(id: number): Promise<ProjectTree> {
  return request(`/api/building-parts/${id}`, { method: "DELETE" });
}

export function renameBuildingPart(id: number, name: string): Promise<ProjectTree> {
  return request(`/api/building-parts/${id}`, {
    method: "PATCH",
    body: JSON.stringify({ name }),
  });
}

export function moveDeviceToBuildingPart(
  deviceId: number,
  partId: number | null,
): Promise<ProjectTree> {
  return request("/api/move-device-to-building-part", {
    method: "POST",
    body: JSON.stringify({ deviceId, partId }),
  });
}

// `direction` is `"Send"` or `"Receive"` — the same strings
// `ComObjectNode.links[].direction` already carries (both sides format
// `knx_core::Direction`'s `Debug` form), so a link read from the Inspector
// can be passed straight back to `unlinkComObject` unchanged.
export function linkComObject(
  comObjectId: number,
  gaId: number,
  direction: string,
): Promise<ProjectTree> {
  return request("/api/group-links", {
    method: "POST",
    body: JSON.stringify({ comObjectId, gaId, direction }),
  });
}

export function unlinkComObject(
  comObjectId: number,
  gaId: number,
  direction: string,
): Promise<ProjectTree> {
  return request("/api/group-links", {
    method: "DELETE",
    body: JSON.stringify({ comObjectId, gaId, direction }),
  });
}

export function createArea(name: string, address: number): Promise<ProjectTree> {
  return request("/api/areas", { method: "POST", body: JSON.stringify({ name, address }) });
}

export function deleteArea(id: number): Promise<ProjectTree> {
  return request(`/api/areas/${id}`, { method: "DELETE" });
}

export function createLine(
  areaId: number,
  name: string,
  address: number,
  mediumRef: string,
): Promise<ProjectTree> {
  return request("/api/lines", {
    method: "POST",
    body: JSON.stringify({ areaId, name, address, mediumRef }),
  });
}

export function deleteLine(id: number): Promise<ProjectTree> {
  return request(`/api/lines/${id}`, { method: "DELETE" });
}

export function moveDeviceToLine(deviceId: number, lineId: number | null): Promise<ProjectTree> {
  return request("/api/move-device", {
    method: "POST",
    body: JSON.stringify({ deviceId, lineId }),
  });
}

// Server-only DTOs (`apps/knx-server/src/routes.rs`'s `CatalogManufacturerDto`/
// `CatalogItemDto`) — not `knx-projection` types, so no `ts-rs` binding exists
// for them; hand-written to match the JSON shape, same as every request
// body's plain object literal above.
export interface CatalogManufacturer {
  id: string;
  name: string | null;
}

export interface CatalogItem {
  id: string;
  manufacturerId: string;
  name: string | null;
  number: string | null;
  visibleDescription: string | null;
  productRefId: string | null;
  hardware2programRefId: string | null;
}

export interface CatalogInstallMember {
  path: string;
  // Not a taxonomy to switch on: the server already qualifies the one role
  // that could be misread as a passed check ("Signature (stored, not
  // verified)" — KNOWN_LIMITATIONS.md §85). Display it, don't parse it.
  role: string;
  sha256: string;
  size: number;
}

export interface CatalogInstallReport {
  sha256: string;
  scheme: number;
  skipped: boolean;
  members: CatalogInstallMember[];
  unknown: number;
  conflicts: number;
}

export interface CreationDiagnostic {
  kind:
    | "programlessProduct"
    | "ambiguousDpt"
    | "comObjectRefMissing"
    | "programRefMissing"
    | "dynamicOrModuleNotEvaluated";
  catalogItemId?: string;
  refId?: string;
  alternatives?: string[];
  programRef?: string;
  programId?: string;
  /** Ready-to-display English sentence built server-side. The UI no
   * longer prefers this: `CatalogBrowser.tsx`'s `describeCreationDiagnostic`
   * re-composes the sentence from the structured fields above so it can
   * render in the active UI language, and only falls back to this raw
   * string for a `kind` it doesn't recognise yet (see that function's own
   * comment, the D4 exception). */
  detail: string;
}

export interface CreateDeviceResponse {
  tree: ProjectTree;
  diagnostics: CreationDiagnostic[];
}

export function catalogManufacturers(): Promise<CatalogManufacturer[]> {
  return request("/api/catalog/manufacturers");
}

export function catalogItems(
  manufacturer?: string,
  search?: string,
  language?: string | null,
): Promise<CatalogItem[]> {
  const params = new URLSearchParams();
  if (manufacturer) params.set("manufacturer", manufacturer);
  if (search) params.set("search", search);
  if (language) params.set("language", language);
  const qs = params.toString();
  return request(`/api/catalog/items${qs ? `?${qs}` : ""}`);
}

// `ProductLanguageDto` (apps/knx-server/src/routes.rs) — server-local, no
// `ts-rs` binding, hand-written to match its `#[serde(rename_all =
// "camelCase")]` shape, same convention as `CatalogManufacturer`/
// `CatalogItem` above. `rows` is a plain count, not a completeness
// judgement — the Settings panel's label just reports it verbatim.
export interface ProductLanguage {
  language: string;
  rows: number;
}

// Returns `[]` when no product database is installed (the server's own
// doc comment on `product_languages`), not an error — callers render that
// as "no languages available" rather than treating it as a failed fetch.
export function productLanguages(): Promise<ProductLanguage[]> {
  return request("/api/product-languages");
}

/// What the running build is: `<version>[+g<sha>]`, straight from
/// `knx-server`'s `version_string()` and therefore from its manifest plus
/// the commit the build knew about. Carries no crate name — the About
/// dialog puts the *application's* name in front of it. Fetched rather
/// than imported from `package.json`, which versions the frontend package
/// and nothing else.
export function serverVersion(): Promise<{ version: string }> {
  return request("/api/version");
}

/**
 * `GET /api/auth/status`'s answer (ADR-0026). `required` decides whether a
 * login screen may exist at all — the desktop shell runs with
 * authentication off and answers `{ required: false, authenticated: true }`,
 * which is the branch that keeps a password prompt off a machine that never
 * had one. The call deliberately does not refresh the session's idle clock,
 * so asking is free and changes nothing.
 */
export interface AuthStatus {
  required: boolean;
  authenticated: boolean;
}

export function authStatus(): Promise<AuthStatus> {
  return request("/api/auth/status");
}

/**
 * `POST /api/auth/login`. Resolves once the session cookie is set (the
 * cookie is `HttpOnly`, so nothing here can read it and nothing here needs
 * to); rejects with status 401 for a wrong password — after the server's
 * deliberate penalty delay, which is why the caller must not let a second
 * attempt start while this one is in flight — or 400 if this server has no
 * password configured at all.
 *
 * The password goes in the body and nowhere else: never a query string,
 * never a header, never a log line.
 */
export function login(password: string): Promise<{ authenticated: boolean }> {
  return request("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ password }),
  });
}

/** `POST /api/auth/logout`. Idempotent, and answers 200 even with no session. */
export function logout(): Promise<{ authenticated: boolean }> {
  return request("/api/auth/logout", { method: "POST" });
}

/// Uses multipart directly rather than `request()`: setting JSON's
/// `Content-Type` on a FormData request would remove the required boundary.
export async function installProductPackage(file: File): Promise<CatalogInstallReport> {
  const form = new FormData();
  form.append("file", file);
  const response = await fetch("/api/catalog/install", { method: "POST", body: form });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    noteRefusal("/api/catalog/install", response.status);
    throw requestError(response.status, body?.error ?? `${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<CatalogInstallReport>;
}

export function createDevice(
  lineId: number | null,
  catalogItemId: string,
  name: string,
): Promise<CreateDeviceResponse> {
  return request("/api/devices", {
    method: "POST",
    body: JSON.stringify(lineId === null ? { catalogItemId, name } : { lineId, catalogItemId, name }),
  });
}

export function deleteDevice(id: number): Promise<ProjectTree> {
  return request(`/api/devices/${id}`, { method: "DELETE" });
}

export function batchDeleteDevices(ids: number[]): Promise<ProjectTree> {
  return request("/api/devices/batch-delete", {
    method: "POST",
    body: JSON.stringify({ ids }),
  });
}

export function batchDeleteGroupAddresses(ids: number[]): Promise<ProjectTree> {
  return request("/api/group-addresses/batch-delete", {
    method: "POST",
    body: JSON.stringify({ ids }),
  });
}

export function batchMoveDevicesToLine(
  deviceIds: number[],
  lineId: number | null,
): Promise<ProjectTree> {
  return request("/api/devices/batch-move-line", {
    method: "POST",
    body: JSON.stringify({ deviceIds, lineId }),
  });
}

export function batchMoveDevicesToBuildingPart(
  deviceIds: number[],
  buildingPartId: number | null,
): Promise<ProjectTree> {
  return request("/api/devices/batch-move-building-part", {
    method: "POST",
    body: JSON.stringify({ deviceIds, buildingPartId }),
  });
}

// `CsvProblemDto` (apps/knx-server/src/routes.rs) — server-local, no
// `ts-rs` binding, hand-written to match its `#[serde(rename_all =
// "camelCase")]` JSON shape. Doubles as an export warning and an import
// row-level problem, same as the Rust side reuses one type for both.
export interface CsvProblem {
  row: number | null;
  severity: "error" | "warning";
  detail: string;
}

export interface CsvExportReport {
  warnings: CsvProblem[];
}

// "KNXBench group-address CSV v1" (crates/knx-csv, design
// docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md)
// — a format this project defines and owns, not an ETS export. `path` is a
// fresh write target resolved server-side, like every route that writes a
// file the user named.
export function exportGroupAddressesCsv(path: string): Promise<CsvExportReport> {
  return request("/api/group-addresses/csv-export", {
    method: "POST",
    body: JSON.stringify({ path }),
  });
}

export interface IgnoredColumn {
  name: string;
  reason: "exportOnly" | "unknown";
}

export interface CsvImportReport {
  separator: string;
  rowsRead: number;
  created: number;
  updated: number;
  unchanged: number;
  ignoredColumns: IgnoredColumn[];
  problems: CsvProblem[];
}

export interface CsvImportResponse {
  tree: ProjectTree;
  report: CsvImportReport;
}

// A file that parses but contains a row-level error rejects with a 400
// (`request()` throws, `tree` never reaches the caller) and leaves the
// open project untouched — see `import_group_addresses_csv` in
// apps/knx-server/src/routes.rs.
export function importGroupAddressesCsv(path: string): Promise<CsvImportResponse> {
  return request("/api/group-addresses/csv-import", {
    method: "POST",
    body: JSON.stringify({ path }),
  });
}

// `DocumentationWarningDto`/`DocumentationExportReportDto`
// (apps/knx-server/src/routes.rs) — server-local, no `ts-rs` binding,
// hand-written to match their `#[serde(rename_all = "camelCase")]` JSON
// shape (a no-op here: every field is already a single word). The export
// itself writes a self-contained "project documentation" HTML file —
// never an "ETS report", since no ETS-produced sample exists anywhere in
// this repository to be compatible with.
export interface ReportWarning {
  location: string;
  detail: string;
}

export interface DocumentationExportReport {
  warnings: ReportWarning[];
}

export function exportDocumentation(path: string): Promise<DocumentationExportReport> {
  return request("/api/project/documentation-export", {
    method: "POST",
    body: JSON.stringify({ path }),
  });
}

// `LogEntry` (apps/knx-server/src/session_log.rs) — server-local, no
// `ts-rs` binding, hand-written to match its `#[serde(rename_all =
// "camelCase")]` JSON shape (same pattern as `CatalogInstallReport` above).
export interface LogEntry {
  timestamp: string;
  severity: "error" | "warning" | "info";
  source: string;
  message: string;
  location: string | null;
  detail: string | null;
  diagnostic?: SettingsDiagnostic;
}

export function getSessionLog(): Promise<LogEntry[]> {
  return request("/api/log");
}

// ---------------------------------------------------------------------
// `POST /api/debug-report` (T29). Hand-written to match
// `apps/knx-server/src/debug_report_routes.rs`'s camelCase DTOs, same
// convention as `LogEntry` above.
//
// Nothing here uploads anything: the route builds a bundle, optionally
// writes it to a path the user picked, and returns it. `path: null` is the
// preview the GitHub-issue button uses — it needs `reportMarkdown` and has
// no business creating a file nobody asked for, which is why `written`
// exists rather than being inferred from "the call did not throw".
// ---------------------------------------------------------------------

export interface DebugReportRequest {
  path: string | null;
  description: string;
  includeLog: boolean;
  includeProjectSummary: boolean;
  includeBusTelegrams: boolean;
  client: {
    appVersion: string;
    shell: string;
    uiLanguage: string;
    theme: string;
  };
}

export interface DebugReportFile {
  name: string;
  bytes: number;
}

export interface DebugReport {
  written: boolean;
  path: string | null;
  files: DebugReportFile[];
  reportMarkdown: string;
}

export function createDebugReport(body: DebugReportRequest): Promise<DebugReport> {
  return request("/api/debug-report", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

// ---------------------------------------------------------------------
// `POST /api/project/diff` (T14) DTOs (`apps/knx-server/src/routes.rs`,
// search for "diff"). `knx_diff::*` has no `ts-rs` binding (the crate
// deliberately does not derive `Serialize` — its own module doc), so
// every interface below is hand-written to match its DTO's
// `#[serde(rename_all = "camelCase")]` shape, same convention as
// `CsvProblem` above. `EntityTable<K, F>`/`EntityChange<K, F>`/
// `AmbiguityNote<K>` mirror `EntityTableDto`/`EntityChangeDto`/
// `AmbiguityNoteDto`, generic there for the same reason they are generic
// here. Devices get their own non-generic `DeviceTable`/`DeviceChange`,
// mirroring `DeviceTableDto`/`DeviceChangeDto` — nesting a device's
// communication objects/parameters inside a `Fields` type shared by a
// generic `EntityChange` would duplicate the nested diff meaninglessly,
// the same reasoning `knx-diff` itself and `routes.rs` both record.
// ---------------------------------------------------------------------

// `MatchKindDto` — mirrors `knx_diff::MatchKind`.
export type MatchKind = "etsId" | "naturalKey";

// `EntityStatusDto` — mirrors `knx_diff::EntityStatus`, an installation's
// own added/removed/matched status.
export type EntityStatus = "added" | "removed" | "matched";

// `FieldChangeDto` — mirrors `knx_diff::FieldChange`, one changed
// scalar field with its old and new value already stringified.
export interface FieldChange {
  field: string;
  left: string;
  right: string;
}

// `EntityChangeDto<K, F>` — mirrors `knx_diff::EntityChange<K, F>`.
export interface EntityChange<K, F> {
  key: K;
  matchedBy: MatchKind;
  left: F;
  right: F;
  changedFields: string[];
  fieldChanges: FieldChange[];
}

// `AmbiguityNoteDto<K>` — mirrors `knx_diff::AmbiguityNote<K>`: a key
// that matched more than one candidate on at least one side, so it could
// not be placed in `changed` and instead sits in both `added`/`removed`.
export interface AmbiguityNote<K> {
  key: K;
  leftCandidates: number;
  rightCandidates: number;
}

// `EntityTableDto<K, F>` — mirrors `knx_diff::EntityTable<K, F>`, used
// for every entity kind except devices.
export interface EntityTable<K, F> {
  added: [K, F][];
  removed: [K, F][];
  changed: EntityChange<K, F>[];
  ambiguous: AmbiguityNote<K>[];
}

// `AreaKeyDto`/`AreaFieldsDto` — mirror `knx_diff::AreaKey`/`AreaFields`.
export interface AreaKey {
  address: number;
}

export interface AreaFields {
  name: string;
  completion: string;
}

// `LineKeyDto`/`LineFieldsDto` — mirror `knx_diff::LineKey`/`LineFields`.
export interface LineKey {
  areaAddress: number;
  lineAddress: number;
}

export interface LineFields {
  name: string;
  mediumRef: string;
  domainAddress: string | null;
  domainAddressIsChecked: boolean | null;
  ipRoutingMulticastAddress: string | null;
  multicastTtl: number | null;
  completion: string;
  area: AreaKey | null;
}

// `BuildingPartKeyDto`/`BuildingPartFieldsDto` — mirror
// `knx_diff::BuildingPartKey`/`BuildingPartFields`.
export interface BuildingPartKey {
  path: string[];
}

export interface BuildingPartFields {
  name: string;
  number: string | null;
  kind: string;
  completion: string;
  defaultLine: LineKey | null;
}

// `DeviceKeyDto` — mirrors `knx_diff::DeviceKey`.
export interface DeviceKey {
  etsId: string | null;
  address: string | null;
}

// `CommissioningStateDto` — mirrors `knx_core::CommissioningState`.
export interface CommissioningState {
  completion: string;
  individualAddressLoaded: boolean;
  applicationProgramLoaded: boolean;
  parametersLoaded: boolean;
  communicationPartLoaded: boolean;
  mediumConfigLoaded: boolean;
  lastModified: string | null;
  lastDownload: string | null;
  broken: boolean;
}

// `DeviceFieldsDto` — mirrors `knx_diff::DeviceFields`.
export interface DeviceFields {
  name: string;
  description: string | null;
  address: string | null;
  productRef: string;
  programRef: string;
  commissioning: CommissioningState;
  line: LineKey | null;
  building: BuildingPartKey | null;
}

// `GroupRangeKeyDto`/`GroupRangeFieldsDto` — mirror
// `knx_diff::GroupRangeKey`/`GroupRangeFields`.
export interface GroupRangeKey {
  start: number;
  end: number;
}

export interface GroupRangeFields {
  name: string;
  start: number;
  end: number;
  parent: GroupRangeKey | null;
}

// `GroupAddressKeyDto`/`GroupAddressFieldsDto` — mirror
// `knx_diff::GroupAddressKey`/`GroupAddressFields`.
export interface GroupAddressKey {
  etsId: string | null;
  address: string;
}

export interface GroupAddressFields {
  name: string;
  central: boolean;
  unfiltered: boolean;
  range: GroupRangeKey | null;
}

// `ComObjectKeyDto` — mirrors `knx_diff::ComObjectKey`.
export interface ComObjectKey {
  device: DeviceKey;
  number: number;
}

// `ComObjectLinkDto` — one `ComObjectFields::links` entry: a group
// address plus the direction it is used in ("Send"/"Receive", `Direction`'s
// `Debug` form, same convention `ProjectTree`'s own group links use).
export interface ComObjectLink {
  groupAddress: GroupAddressKey;
  direction: string;
}

// `ComObjectFieldsDto` — mirrors `knx_diff::ComObjectFields`.
export interface ComObjectFields {
  text: string | null;
  description: string | null;
  dpt: string | null;
  read: boolean | null;
  write: boolean | null;
  transmit: boolean | null;
  update: boolean | null;
  communication: boolean | null;
  readOnInit: boolean | null;
  links: ComObjectLink[];
  moduleInstance: string | null;
}

// `ParameterKeyDto`/`ParameterFieldsDto` — mirror
// `knx_diff::ParameterKey`/`ParameterFields`.
export interface ParameterKey {
  device: DeviceKey;
  etsId: string;
}

export interface ParameterFields {
  raw: string;
}

// `DeviceTableDto` — devices' own, non-generic table (see module comment
// above for why).
export interface DeviceTable {
  added: [DeviceKey, DeviceFields][];
  removed: [DeviceKey, DeviceFields][];
  changed: DeviceChange[];
  ambiguous: AmbiguityNote<DeviceKey>[];
}

// `DeviceChangeDto` — a changed device, plus its nested communication
// object and parameter diffs.
export interface DeviceChange {
  key: DeviceKey;
  matchedBy: MatchKind;
  left: DeviceFields;
  right: DeviceFields;
  changedFields: string[];
  fieldChanges: FieldChange[];
  comObjects: EntityTable<ComObjectKey, ComObjectFields>;
  parameters: EntityTable<ParameterKey, ParameterFields>;
}

// `InstallationDiffDto` — mirrors `knx_diff::InstallationDiff`.
export interface InstallationDiff {
  id: number;
  status: EntityStatus;
  fieldChanges: FieldChange[];
  areas: EntityTable<AreaKey, AreaFields>;
  lines: EntityTable<LineKey, LineFields>;
  devices: DeviceTable;
  groupRanges: EntityTable<GroupRangeKey, GroupRangeFields>;
  groupAddresses: EntityTable<GroupAddressKey, GroupAddressFields>;
  buildings: EntityTable<BuildingPartKey, BuildingPartFields>;
}

// `ProjectDiffDto` — mirrors `knx_diff::ProjectDiff`, the whole response
// body of `POST /api/project/diff`.
export interface ProjectDiffReport {
  infoChanges: FieldChange[];
  installations: InstallationDiff[];
}

// Compares the server's live, possibly edited, in-memory project against
// the `.knxdb` file at `path` — "what would Save change", never a
// comparison of two files on disk, and never an ETS-parity claim (design
// spec `docs/superpowers/specs/2026-09-10-project-diff-design.md` §7).
export function diffProject(path: string): Promise<ProjectDiffReport> {
  return request("/api/project/diff", {
    method: "POST",
    body: JSON.stringify({ path }),
  });
}

// ---------------------------------------------------------------------
// `/api/bus/*` (T15 task 3, `apps/knx-server/src/bus_routes.rs`) DTOs.
// Server-local, no `ts-rs` binding — same hand-written convention as
// `LogEntry` above, field names matching that file's `#[serde(rename_all
// = "camelCase")]` DTO structs exactly (design spec
// `docs/superpowers/specs/2026-09-11-group-monitor-design.md` §4.3 states
// the wire shape in prose; where its JSON *examples* disagree with the
// actual Rust — the DPT text below is the one confirmed case — the Rust
// wins, per that file's own doc comment on `WriteRequest`).
// ---------------------------------------------------------------------

// `DecodedValueDto` (bus_routes.rs, struct `DecodedValueDto`) — D4's
// four-way decode outcome. `dpt`/`error` are each only ever present for
// one `kind` (`"value"`/`"error"` respectively) — modelled as always-
// optional rather than a discriminated union per `kind`, because nothing
// here needs the narrowing and a union would just move the same
// `undefined` checks into every call site. `dpt`, when present, is
// `DptRef`'s `Display` text (`"DPST-1-1"`, not the dotted `"1.001"` the
// design spec's own §4.3 example shows — confirmed against the shipped
// encoder, not invented here) — rendered exactly as received, no
// dotted-notation prettifier added by this task.
export interface BusDecodedValue {
  kind: "value" | "unresolved" | "conflict" | "error";
  dpt?: string;
  text: string;
  error?: string;
}

// `TelegramRowDto` (bus_routes.rs, struct `TelegramRowDto`). `service` is
// typed as a plain `string`, not a literal union of
// `ApplicationService`'s four variant names: `bus.rs`'s
// `push_closed_marker` also pushes a row with `service: "SessionClosed"`,
// a synthetic marker outside that enum on purpose (its own doc comment:
// "cannot be mistaken for real bus traffic by anything that later renders
// this row") — a union typed to only the four real services would make
// that marker a type error the moment it arrived.
export interface BusTelegramRow {
  seq: number;
  timestamp: string;
  source: string;
  destination: string;
  destinationName: string | null;
  service: string;
  rawPayload: string | null;
  decoded: BusDecodedValue | null;
}

// `StartResponse` (bus_routes.rs).
export interface BusMonitorStartResponse {
  sessionId: number;
  serverIncarnation: string;
  assignedAddress: string;
}

// `StopResponse` (bus_routes.rs). `serverIncarnation` scopes the reusable
// numeric id, as it does on start/poll. `warning` is `skip_serializing_if`
// there, hence optional here — present only when the drain task's own
// teardown panicked after an otherwise-successful stop (its doc comment:
// "a panic surfaced by the server and then swallowed by the UI is worse
// than not surfacing it at all"). This is the one field the design spec's
// §4.3 prose does not mention at all; it was added on review during task
// 3 and is real on the wire.
export interface BusMonitorStopResponse {
  sessionId: number;
  serverIncarnation: string;
  telegramCount: number;
  droppedCount: number;
  warning?: string;
}

// `TelegramsResponse` (bus_routes.rs). `droppedBefore` is the buffer's
// running total at response time, not scoped to `since` — the caller
// compares it against what it already knew to notice a fresh gap (see
// `BusMonitorPanel.tsx`).
export interface BusMonitorTelegramsResponse {
  sessionId: number;
  serverIncarnation: string;
  status: "active" | "closed";
  nextSince: number;
  droppedBefore: number;
  telegrams: BusTelegramRow[];
}

// `WriteRequest`/`WriteResponse` (bus_routes.rs). `writeBusValue()` is
// task 5's compose form's own binding; the DTO is hand-written here
// alongside its three siblings since all four routes share this file's
// provenance comment and none of the other three has anywhere better to
// live either.
//
// `decodedEcho` (task 27) reuses `BusDecodedValue` — `WriteResponse`'s
// field is that same DTO on the wire (`decoded_echo: DecodedValueDto` in
// Rust) — decoded from the bytes the write actually sent, never from the
// request's own `value`, so it always carries `kind: "value" | "error"`
// in practice (the DPT this route decodes against is always already
// resolved by the time it runs, never `"unresolved"`/`"conflict"`).
export interface BusWriteResponse {
  encodedPayload: string;
  service: "GroupValueWrite";
  decodedEcho: BusDecodedValue;
}

export type DptInputFormat = "canonical" | "decimal" | "hexadecimal" | "binary" | "text";

// `DiscoveredInterfaceDto`/`DiscoverResponse` (bus_routes.rs, T25). One
// entry per KNX-compatible interface that answered the multicast
// `SEARCH_REQUEST`. `controlEndpoint` is already in the `host:port` shape
// `startBusMonitor` wants, so a found interface goes straight into the
// gateway field with no reassembly.
export interface BusDiscoveredInterface {
  controlEndpoint: string;
  individualAddress: string;
  friendlyName: string;
  supportsTunnelling: boolean;
}

export interface BusDiscoverResponse {
  interfaces: BusDiscoveredInterface[];
}

export interface LineScanRequest {
  gateway: string;
  area: number;
  line: number;
  firstDevice: number;
  lastDevice: number;
  excluded: string[];
  responseTimeoutMs: number;
  interProbePauseMs: number;
}

export interface LineScanEstimate {
  candidateCount: number;
  omittedAddresses: string[];
  responseTimeoutMs: number;
  vacantConfirmations: number;
  interProbePauseMs: number;
  worstCaseMs: number;
}

export type LineScanOutcome =
  | { kind: "occupied"; maskVersion: number | null }
  | { kind: "occupiedBusy" }
  | { kind: "occupiedSilent" }
  | { kind: "vacant" }
  | { kind: "indeterminate" }
  | { kind: "selfAddress" };

export interface LineScanResult {
  address: string;
  outcome: LineScanOutcome;
}

export interface LineScanStartResponse {
  sessionId: number;
  estimate: LineScanEstimate;
}

export interface LineScanResultsResponse {
  sessionId: number;
  status: "running" | "completed" | "cancelled" | "failed";
  error: string | null;
  nextSince: number;
  completedCount: number;
  totalCount: number;
  omittedAddresses: string[];
  excludedAddresses: string[];
  results: LineScanResult[];
}

export interface LineScanComparison {
  unexpected: string[];
  missing: string[];
  excludedInProject: string[];
}

export function startBusMonitor(gateway: string): Promise<BusMonitorStartResponse> {
  return request("/api/bus/monitor/start", { method: "POST", body: JSON.stringify({ gateway }) });
}

export function stopBusMonitor(): Promise<BusMonitorStopResponse> {
  return request("/api/bus/monitor/stop", { method: "POST" });
}

// `since` defaults to `0` server-side too (`TelegramsQuery.since:
// Option<u64>`) — always sent explicitly here so a caller never has to
// remember that omitting it means "from the start."
export function pollBusTelegrams(since: number): Promise<BusMonitorTelegramsResponse> {
  return request(`/api/bus/monitor/telegrams?since=${since}`);
}

// `POST`, matching the route (`bus_routes.rs`'s `discover_interfaces`):
// the search puts a multicast datagram on the network, and a `GET` would
// be fair game for prefetching. An empty `interfaces` array is a normal
// success, never an error — see `busDiscovery.ts`.
export function discoverBusInterfaces(): Promise<BusDiscoverResponse> {
  return request("/api/bus/discover", { method: "POST" });
}

export function estimateLineScan(requestBody: LineScanRequest): Promise<LineScanEstimate> {
  return request("/api/bus/scan/estimate", {
    method: "POST",
    body: JSON.stringify(requestBody),
  });
}

export function startLineScan(requestBody: LineScanRequest): Promise<LineScanStartResponse> {
  return request("/api/bus/scan/start", {
    method: "POST",
    body: JSON.stringify(requestBody),
  });
}

export function pollLineScan(since: number, sessionId?: number): Promise<LineScanResultsResponse> {
  const expected = sessionId === undefined ? "" : `&sessionId=${sessionId}`;
  return request(`/api/bus/scan/results?since=${since}${expected}`);
}

export function cancelLineScan(sessionId: number): Promise<LineScanResultsResponse> {
  return request(`/api/bus/scan/cancel?sessionId=${sessionId}`, { method: "POST" });
}

export function compareLineScan(sessionId: number): Promise<LineScanComparison> {
  return request(`/api/bus/scan/comparison?sessionId=${sessionId}`);
}

export function reconcileLineScan(
  sessionId: number,
  unexpected: string[],
  missing: string[],
): Promise<ProjectTree> {
  return request("/api/bus/scan/reconcile", {
    method: "POST",
    body: JSON.stringify({ sessionId, unexpected, missing }),
  });
}

export function writeBusValue(
  destination: string,
  dpt: string | null,
  value: string,
  inputFormat: DptInputFormat | null,
): Promise<BusWriteResponse> {
  return request("/api/bus/write", {
    method: "POST",
    body: JSON.stringify({ destination, dpt, inputFormat, value }),
  });
}

// ---------------------------------------------------------------------
// `/api/device/{id}/parameters` (T18 slice 3, task 4). `ParameterPanelDto`
// and its nested DTOs (`apps/knx-server/src/routes.rs`) are server-local
// (`#[derive(Serialize)]`, not `ts-rs` — that file's own doc comment on
// `ParameterPanelDto` names the coordinator's ruling on DTO placement),
// so these are hand-written to match the `#[serde(rename_all =
// "camelCase")]` wire shape, same convention as `CsvProblem`/
// `CreationDiagnostic` above. `displayOrder` rides along for completeness
// but is informational only (design D22) — nothing here sorts by it; the
// server already returns `sections`/`fields` in the order the panel must
// render them.
// ---------------------------------------------------------------------

export interface ModuleScope {
  moduleNode: number;
  moduleId: string | null;
  moduleDefId: string;
}

export interface EnumOption {
  value: string;
  text: string | null;
}

export interface ParameterField {
  etsId: string;
  name: string | null;
  text: string | null;
  kind: string;
  value: string | null;
  valueSource: string;
  editable: boolean;
  min: string | null;
  max: string | null;
  enumOptions: EnumOption[];
  displayOrder: number | null;
  access: string | null;
  // The `etsId` a write must actually name (T18 slice 4, design D43):
  // `null` exactly when `editable` is `false`, otherwise the id `POST`
  // has to send — `etsId` above for an unscoped field, a module-qualified
  // id for an editable module-scoped one. `ParameterPanel.tsx` writes
  // this, never `etsId`.
  writeEtsId: string | null;
}

export interface ParameterSection {
  scope: ModuleScope | null;
  fields: ParameterField[];
}

export interface StaleParameter {
  etsId: string;
  raw: string;
}

/**
 * Every `kind` `ParameterDiagnosticKindDto` (`apps/knx-server/src/routes.rs`)
 * can serialize, mirrored here by hand the same way `CreationDiagnostic`'s
 * `kind` union is (no ts-rs binding for this DTO). None of these carry a
 * dynamic value of their own — see that Rust type's doc comment — so
 * `ParameterPanel.tsx`'s `describeParameterDiagnosticMessage` only ever
 * needs the tag itself, never a payload field.
 */
export type ParameterDiagnosticKind =
  | "parametersUnreadable"
  | "duplicateUnscopedValue"
  | "duplicateModuleScopedValue"
  | "duplicateModuleId"
  | "noModuleInstanceMatch"
  | "ambiguousModuleInstance"
  | "malformedModuleInstanceId"
  | "noBranchMatched"
  | "unparsableTest"
  | "unresolvedParamRef"
  | "nonNumericValue"
  | "unexpectedTypeNoneShape"
  | "unrecognizedNode"
  | "moduleDefNotFound"
  | "moduleCycleDetected"
  | "moduleNestingTooDeep"
  | "moduleExpansionBudgetExhausted"
  | "missingValue"
  | "moduleWithoutId"
  | "moduleArgumentNotBound"
  | "unsupportedModuleArgumentKind"
  | "unresolvedTextPlaceholder";

export interface ParameterDiagnostic {
  scope: ModuleScope | null;
  /** Same trust boundary as `CreationDiagnostic.kind` (D4 exception,
   * `CatalogBrowser.tsx`): this type is a promise about the wire shape
   * this build's server sends, not something TS enforces at the network
   * boundary. A future server release adding a twenty-third diagnostic would
   * hand an older frontend a `kind` outside this union at runtime; that
   * frontend's `describeParameterDiagnosticMessage` falls back to
   * `message` verbatim (English) rather than rendering nothing. */
  kind: ParameterDiagnosticKind;
  /** KNOWN_LIMITATIONS.md §66: the banner headline, now translatable —
   * `ParameterPanel.tsx` renders it through `describeParameterDiagnosticMessage`,
   * not verbatim. This field is the untranslated fallback for an
   * unrecognised `kind` only. */
  message: string;
  /** Always English, deliberately: carries the dynamic ids/counts a
   * translated sentence would have nowhere to put, and exists for
   * copy-pasting into a bug report (the "Copy details" button), not for
   * reading in the active UI language — see KNOWN_LIMITATIONS.md §66's
   * boundary rule. */
  detail: string;
}

// The read model (`GET`) and a successful write's response (`POST`) ride
// the same DTO — design D24's "same response, no second `GET`".
export interface ParameterPanel {
  programId: string | null;
  sections: ParameterSection[];
  stale: StaleParameter[];
  diagnostics: ParameterDiagnostic[];
  // The server's own freshly rebuilt tree (T3 fix round 1, item 6) — `null`
  // from the plain `GET`, which runs no command and has nothing fresher to
  // offer; always present on a successful `POST`, since `apply()` on the
  // server already built it from the genuine post-write `CommandStack`.
  tree: ProjectTree | null;
}

// `language` selects which stored translation row the server substitutes
// into a field's `text` (empty/omitted means the package's untranslated
// default) — it never changes which parameters exist or what a write
// stores, only the wording the read model comes back with. Built the same
// way `catalogItems` above builds its optional query string, so the two
// don't drift into two different conventions for the same thing.
function languageQuery(language?: string | null): string {
  const params = new URLSearchParams();
  if (language) params.set("language", language);
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

export function deviceParameters(
  deviceId: number,
  language?: string | null,
): Promise<ParameterPanel> {
  return request(`/api/device/${deviceId}/parameters${languageQuery(language)}`);
}

export function setParameterValue(
  deviceId: number,
  etsId: string,
  raw: string,
  language?: string | null,
): Promise<ParameterPanel> {
  return request(`/api/device/${deviceId}/parameters${languageQuery(language)}`, {
    method: "POST",
    body: JSON.stringify({ etsId, raw }),
  });
}

export function undo(): Promise<ProjectTree> {
  return request("/api/undo", { method: "POST" });
}

export function redo(): Promise<ProjectTree> {
  return request("/api/redo", { method: "POST" });
}

/// Unwraps the message from an error thrown by `request()` (or anything
/// else `Error`-shaped); falls back to `String(e)` for non-`Error` throws.
/// Callers used to do `String(e)` directly, back when Tauri's `invoke()`
/// rejected with a bare string — now that every API error is a real
/// `Error`, that produced a doubled `Error: <message>` in the UI.
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/// The HTTP status `request()` attached to an error it threw (see
/// `requestError`), or `undefined` for anything that did not come from
/// `request()` at all (a thrown non-`Error`, a bug elsewhere). Duck-typed
/// rather than an `instanceof` check against an exported error class — see
/// `requestError`'s own comment on why.
export function errorStatus(e: unknown): number | undefined {
  if (e instanceof Error && "status" in e && typeof (e as { status: unknown }).status === "number") {
    return (e as { status: number }).status;
  }
  return undefined;
}
