/** Pure rules of the Report problem wizard: O1 defaults, requests, sizes, client state. */
import type {
  BundleQuery, BundleRecordingSummary, ContentMarkers, Offer, OperationRef, ProblemType, SectionKind, SectionPreview,
} from "./diagnosticBundleShape";
import { PROBLEM_TYPES, SECTION_KINDS } from "./diagnosticBundleShape";
import type { BundlePreview, ProjectContext, ReviewedBundleQuery } from "./diagnosticBundleShape";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MessageKey } from "./messages/en";
import type { ToastNotification } from "./toast";

/** How a shortcut opens the wizard: problem type, device and operation preselected. */
export interface ReportProblemRequest {
  problemType?: ProblemType;
  deviceId?: number | null;
  operation?: OperationRef | null;
}

type Default = Offer | "notOffered";

/**
 * Owner table O1 verbatim; columns: programming, reading, import, catalogue,
 * UI/other. The server holds the same table (`default_offer`); both are pinned
 * by tests, and the server's answer decides what is actually offered.
 */
const O1: Record<SectionKind, readonly [Default, Default, Default, Default, Default]> = {
  environment: ["fixed", "fixed", "fixed", "fixed", "fixed"],
  sessionLog: ["preselected", "preselected", "preselected", "preselected", "preselected"],
  operation: ["preselected", "preselected", "notOffered", "notOffered", "offered"],
  earlierOperation: ["offered", "offered", "notOffered", "notOffered", "offered"],
  device: ["preselected", "preselected", "notOffered", "offered", "offered"],
  legacyReports: ["preselected", "preselected", "notOffered", "notOffered", "notOffered"],
  importReport: ["notOffered", "notOffered", "preselected", "notOffered", "offered"],
  importFile: ["notOffered", "notOffered", "offered", "notOffered", "notOffered"],
  catalogue: ["offered", "offered", "notOffered", "preselected", "notOffered"],
  project: ["offered", "offered", "offered", "notOffered", "offered"],
  busMonitor: ["offered", "offered", "notOffered", "notOffered", "offered"],
  clientState: ["notOffered", "notOffered", "notOffered", "notOffered", "preselected"],
};

export function defaultOffer(problem: ProblemType, kind: SectionKind): Default {
  return O1[kind][PROBLEM_TYPES.indexOf(problem)];
}

/** What a problem type ticks before the user changes anything. */
export function defaultSelection(problem: ProblemType): SectionKind[] {
  return SECTION_KINDS.filter((kind) => {
    const offer = defaultOffer(problem, kind);
    return offer === "fixed" || offer === "preselected";
  });
}

/** Everything the wizard has chosen so far. */
export interface WizardChoice {
  problemType: ProblemType;
  /** Ticked sections; `earlierOperation` follows `earlier` instead. */
  sections: ReadonlySet<SectionKind>;
  /** `null`: the latest recording that fits the problem type. */
  operation: OperationRef | null;
  deviceId: number | null;
  earlier: readonly string[];
}

export function initialChoice(request: ReportProblemRequest = {}): WizardChoice {
  const problemType = request.problemType ?? "userInterface";
  return {
    problemType,
    sections: new Set(defaultSelection(problemType)),
    operation: request.operation ?? null,
    deviceId: request.deviceId ?? null,
    earlier: [],
  };
}

/** A new problem type takes its own defaults; operation and device stay. */
export function withProblemType(choice: WizardChoice, problemType: ProblemType): WizardChoice {
  if (choice.problemType === problemType) return choice;
  return { ...choice, problemType, sections: new Set(defaultSelection(problemType)), earlier: [] };
}

/** Ticks or unticks one optional section; fixed and unoffered ones cannot change. */
export function toggleSection(choice: WizardChoice, kind: SectionKind, on: boolean): WizardChoice {
  const offer = defaultOffer(choice.problemType, kind);
  if (offer === "fixed" || offer === "notOffered" || kind === "earlierOperation") return choice;
  const sections = new Set(choice.sections);
  if (on) sections.add(kind); else sections.delete(kind);
  return { ...choice, sections };
}

export function toggleEarlier(choice: WizardChoice, id: string, on: boolean): WizardChoice {
  const earlier = choice.earlier.filter((other) => other !== id);
  return { ...choice, earlier: on ? [...earlier, id] : earlier };
}

/**
 * The request for preview, section text and export. Only offered sections are
 * sent; earlier operations count as ticked exactly when some are picked, and
 * the browser describes itself only when the problem type asks for it.
 */
export function buildQuery(choice: WizardChoice, client: Record<string, unknown> | null): BundleQuery {
  const { problemType } = choice;
  const offered = (kind: SectionKind) => defaultOffer(problemType, kind) !== "notOffered";
  const sections = SECTION_KINDS.filter((kind) => {
    if (!offered(kind)) return false;
    if (defaultOffer(problemType, kind) === "fixed") return true;
    if (kind === "earlierOperation") return choice.earlier.length > 0;
    return choice.sections.has(kind);
  });
  const query: BundleQuery = { problemType, sections };
  if (offered("operation") && choice.operation !== null) query.operation = choice.operation;
  if (offered("earlierOperation") && choice.earlier.length > 0) query.earlierRecordingIds = [...choice.earlier];
  if (choice.deviceId !== null) query.deviceId = choice.deviceId;
  if (sections.includes("clientState") && client !== null) query.client = client;
  return query;
}

/** Bind numeric choices to their source project, including the current revision. */
export function reportContext(tree: ProjectTree | null): ProjectContext | undefined {
  return tree?.server_incarnation !== undefined && tree.project_incarnation !== undefined && tree.snapshot_revision !== undefined
    ? { serverIncarnation: tree.server_incarnation, projectIncarnation: tree.project_incarnation, revision: tree.snapshot_revision }
    : undefined;
}

/** Pin resolution separately from choices, so admission cannot cause a preview loop. */
export function reviewedQuery(query: BundleQuery, preview: BundlePreview): ReviewedBundleQuery {
  return {
    ...query,
    context: preview.review.context,
    review: preview.review,
    operation: preview.operationRecordingId !== null ? { recordingId: preview.operationRecordingId }
      : query.operation ?? { selection: "none" },
    ...(preview.deviceId !== null ? { deviceId: preview.deviceId } : {}),
  };
}

/** Bytes in binary units, as the 10 MiB mail hint counts them. */
export function formatBytes(bytes: number, language: string): string {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  let text: string;
  try {
    text = new Intl.NumberFormat(language, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value);
  } catch {
    // A language-pack tag Intl does not accept still gets a number.
    text = unit === 0 ? String(value) : value.toFixed(1);
  }
  return `${text} ${units[unit]}`;
}

/** Bytes per section kind; several earlier operations add up. */
export function bytesByKind(sections: readonly SectionPreview[]): Map<SectionKind, number> {
  const sizes = new Map<SectionKind, number>();
  for (const section of sections) sizes.set(section.kind, (sizes.get(section.kind) ?? 0) + section.bytes);
  return sizes;
}

/** The content markers of a section as message keys, in a fixed order. */
export function markerKeys(markers: ContentMarkers): MessageKey[] {
  const keys: MessageKey[] = [];
  if (markers.addresses) keys.push("reportProblem.marker.addresses");
  if (markers.names) keys.push("reportProblem.marker.names");
  if (markers.parameterValues) keys.push("reportProblem.marker.parameterValues");
  if (markers.networkAddresses) keys.push("reportProblem.marker.networkAddresses");
  return keys;
}

/** The server's fixed English reasons we translate; anything else is shown as sent. */
const REASONS: ReadonlyArray<[string, MessageKey]> = [
  ["no recorded operation fits this problem", "reportProblem.reason.noOperation"],
  ["no operation is chosen", "reportProblem.reason.noOperationChosen"],
  ["no device is chosen", "reportProblem.reason.noDevice"],
  ["no minimised report exists for this operation", "reportProblem.reason.noLegacyReport"],
  ["this server session does not keep the original import file; attach it yourself if needed", "reportProblem.reason.importFile"],
  ["the project has not been saved as a KNXBench project file yet", "reportProblem.reason.projectUnsaved"],
  ["no project is open", "reportProblem.reason.noProject"],
  ["this project carries no import report", "reportProblem.reason.noImportReport"],
  ["this section is withheld until its content is admitted as free of credentials", "reportProblem.reason.withheld"],
  ["the selected operation identifies no single device of this project", "reportProblem.reason.sourceNamesNoDevice"],
  ["the selected diagnostic source is unavailable", "reportProblem.reason.source"],
  ["the originating device source is unavailable", "reportProblem.reason.originatingDevice"],
  ["the retained diagnostic result is unavailable", "reportProblem.reason.retainedMissing"],
  ["the retained diagnostic result has expired", "reportProblem.reason.retainedExpired"],
  ["the diagnostic result is not complete", "reportProblem.reason.retainedUnfinished"],
  ["the diagnostic result is not exportable", "reportProblem.reason.retainedNonExportable"],
  ["the retained support result is unavailable", "reportProblem.reason.supportMissing"],
  ["the retained support result has expired", "reportProblem.reason.supportExpired"],
  ["the support result is not complete", "reportProblem.reason.supportUnfinished"],
  ["the diagnostic recording version is unsupported", "reportProblem.reason.recordingVersion"],
  ["the diagnostic recording result is malformed", "reportProblem.reason.recordingResult"],
];

export function reasonKey(reason: string): MessageKey | null {
  return REASONS.find(([text]) => text === reason)?.[1] ?? null;
}

/** The server's fixed English section notes we translate; other notes are shown as sent. */
const NOTES: ReadonlyArray<[string, MessageKey]> = [
  ["contains device configuration, possibly passwords; recognised KNX keys are redacted; keep private", "reportProblem.note.privateRaw"],
  ["the raw operation recording is withheld; this is the admitted private projection", "reportProblem.note.rawWithheldProjection"],
  ["the raw operation recording is withheld; this is the retained typed result", "reportProblem.note.rawWithheldResult"],
  ["the operation recording is unavailable; this is the admitted private projection", "reportProblem.note.recordingMissingProjection"],
  ["the operation recording is unavailable; this is the retained typed result", "reportProblem.note.recordingMissingResult"],
  ["the recording's last line is incomplete (interrupted write)", "reportProblem.note.tornTail"],
  ["the last saved state of the project; unsaved changes are not included", "reportProblem.note.savedRoot"],
  ["the parameter values could not be projected for this device", "reportProblem.note.noParameters"],
];

export function noteKey(note: string): MessageKey | null {
  return NOTES.find(([text]) => text === note)?.[1] ?? null;
}

const OPERATION_LABELS: Record<string, MessageKey> = {
  "device-diagnostics": "reportProblem.kind.device-diagnostics",
  "device-read": "reportProblem.kind.device-read",
  "download": "reportProblem.kind.download",
  "system2-download": "reportProblem.kind.system2-download",
  "system2-recovery": "reportProblem.kind.system2-recovery",
  "compare": "reportProblem.kind.compare",
  "individual-address-programming": "reportProblem.kind.individual-address-programming",
  "serial-number-address": "reportProblem.kind.serial-number-address",
  "individual-address-reset": "reportProblem.kind.individual-address-reset",
  "service-control": "reportProblem.kind.service-control",
  "line-scan": "reportProblem.kind.line-scan",
  "support-read": "reportProblem.kind.support-read",
  "byte-order-probe": "reportProblem.kind.byte-order-probe",
};
const OUTCOME_LABELS: Record<string, MessageKey> = {
  "succeeded": "reportProblem.outcome.succeeded",
  "failed": "reportProblem.outcome.failed",
  "cancelled": "reportProblem.outcome.cancelled",
  "abandoned": "reportProblem.outcome.abandoned",
};
export const operationKindKey = (kind: string): MessageKey | undefined => OPERATION_LABELS[kind];
export const operationOutcomeKey = (outcome: string): MessageKey | undefined => OUTCOME_LABELS[outcome];

/** The recording the preview resolved, from the list the same answer carries. */
export function resolvedRecording(
  id: string | null,
  recordings: readonly BundleRecordingSummary[],
): BundleRecordingSummary | null {
  return id === null ? null : recordings.find((recording) => recording.id === id) ?? null;
}

/** One line of client state per recent error notification, newest last. */
export interface ClientError {
  timestamp: string;
  source: string;
  message: string;
  detail: string | null;
}

export const RECENT_ERROR_LIMIT = 50;

export function recentErrors(history: readonly ToastNotification[], limit = RECENT_ERROR_LIMIT): ClientError[] {
  return history
    .filter(({ entry }) => entry.severity === "error" || entry.severity === "warning")
    .slice(-limit)
    .map(({ entry }) => ({
      timestamp: entry.timestamp, source: entry.source, message: entry.message, detail: entry.detail ?? null,
    }));
}

export interface ClientFacts {
  appVersion: string;
  shell: "tauri" | "browser";
  uiLanguage: string;
  theme: string;
  viewport: { width: number; height: number };
  uiScale: number;
  recentErrors: ClientError[];
}

/** Bound strings before serializing; budget the complete JSON including loss metadata. */
export function boundedClientFacts(facts: Omit<ClientFacts, "recentErrors">, history: readonly ToastNotification[]): ClientFacts & {
  truncated: boolean; metadataTruncated: boolean; omittedErrorCount: number; truncatedErrorCount: number;
} {
  const total = history.reduce((n, { entry }) => n + Number(entry.severity === "error" || entry.severity === "warning"), 0);
  // Repair only bounded prefixes: never allocate a code-point array for a huge error.
  const prefix = (value: string, limit: number) => value.slice(0, limit).replace(/[\uD800-\uDFFF]/gu, "\uFFFD");
  const bounded = recentErrors(history).map((error) => {
    const entry = {
      timestamp: prefix(error.timestamp, 128), source: prefix(error.source, 256),
      message: prefix(error.message, 2048), detail: error.detail === null ? null : prefix(error.detail, 2048),
    };
    return { ...entry, shortened: entry.timestamp !== error.timestamp || entry.source !== error.source
      || entry.message !== error.message || entry.detail !== error.detail };
  });
  const appVersion = prefix(facts.appVersion, 128), uiLanguage = prefix(facts.uiLanguage, 128), theme = prefix(facts.theme, 128);
  const metadataTruncated = appVersion !== facts.appVersion || uiLanguage !== facts.uiLanguage || theme !== facts.theme;
  const result = { ...facts, appVersion, uiLanguage, theme, recentErrors: [] as ClientError[], truncated: total > 0 || metadataTruncated, metadataTruncated,
    omittedErrorCount: total, truncatedErrorCount: 0 };
  const encoder = new TextEncoder();
  // Newest evidence wins. Only bounded strings participate in repeated serialization.
  for (const { shortened, ...error } of bounded.reverse()) {
    const candidate = { ...result, recentErrors: [error, ...result.recentErrors],
      omittedErrorCount: result.omittedErrorCount - 1, truncatedErrorCount: result.truncatedErrorCount + Number(shortened) };
    candidate.truncated = metadataTruncated || candidate.omittedErrorCount > 0 || candidate.truncatedErrorCount > 0;
    if (encoder.encode(JSON.stringify(candidate)).byteLength > 64 * 1024) break;
    Object.assign(result, candidate);
  }
  result.truncated = metadataTruncated || result.omittedErrorCount > 0 || result.truncatedErrorCount > 0;
  return result;
}

/**
 * The mail draft names the file the preview would write. Without an operation
 * the export's name carries its own timestamp, so the draft names the file
 * actually saved. Both names are plain (letters, digits, `-`, `.`), which the
 * server's percent-encoding leaves unchanged.
 */
export function mailDraftFor(draft: string, previewName: string, savedName: string): string {
  return previewName === savedName ? draft : draft.split(previewName).join(savedName);
}
