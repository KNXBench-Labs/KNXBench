/** Pure rules of the Report problem wizard: O1 defaults, requests, sizes, client state. */
import type {
  BundleQuery, ContentMarkers, Offer, OperationRef, ProblemType, RecordingSummary, SectionKind, SectionPreview,
} from "./diagnosticBundleShape";
import { PROBLEM_TYPES, SECTION_KINDS } from "./diagnosticBundleShape";
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
  if (offered("clientState") && client !== null) query.client = client;
  return query;
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
  ["a chosen earlier recording no longer exists", "reportProblem.reason.earlierGone"],
  ["no device is chosen", "reportProblem.reason.noDevice"],
  ["no minimised report exists for this operation", "reportProblem.reason.noLegacyReport"],
  ["this server session does not keep the original import file; attach it yourself if needed", "reportProblem.reason.importFile"],
  ["the project has not been saved as a KNXBench project file yet", "reportProblem.reason.projectUnsaved"],
  ["the bus monitor is not running", "reportProblem.reason.busMonitorOff"],
  ["the browser did not describe itself", "reportProblem.reason.noClient"],
  ["no project is open", "reportProblem.reason.noProject"],
  ["this project carries no import report", "reportProblem.reason.noImportReport"],
];

export function reasonKey(reason: string): MessageKey | null {
  return REASONS.find(([text]) => text === reason)?.[1] ?? null;
}

/** The recording the preview resolved, from the list the same answer carries. */
export function resolvedRecording(id: string | null, recordings: readonly RecordingSummary[]): RecordingSummary | null {
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

/**
 * The mail draft names the file the preview would write. Without an operation
 * the export's name carries its own timestamp, so the draft names the file
 * actually saved. Both names are plain (letters, digits, `-`, `.`), which the
 * server's percent-encoding leaves unchanged.
 */
export function mailDraftFor(draft: string, previewName: string, savedName: string): string {
  return previewName === savedName ? draft : draft.split(previewName).join(savedName);
}
