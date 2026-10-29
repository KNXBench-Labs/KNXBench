/** Diagnostic bundle wire shapes (DIAG-03): problem types, section kinds and server DTOs. */
// Pure: no fetch, so browser fixtures can import the constants too.

export const PROBLEM_TYPES = [
  "deviceProgramming",
  "deviceReading",
  "projectImport",
  "catalogue",
  "userInterface",
] as const;
export type ProblemType = (typeof PROBLEM_TYPES)[number];

/** The rows of owner table O1, in the server's order (`SectionKind::ALL`). */
export const SECTION_KINDS = [
  "environment",
  "sessionLog",
  "operation",
  "earlierOperation",
  "device",
  "legacyReports",
  "importReport",
  "importFile",
  "catalogue",
  "project",
  "busMonitor",
  "clientState",
] as const;
export type SectionKind = (typeof SECTION_KINDS)[number];

/** `notOffered` never reaches the client: the server lists only offered sections. */
export type Offer = "fixed" | "preselected" | "offered";

/** One operation recording as `GET /api/diagnostics/recordings` lists it. */
export interface RecordingSummary {
  id: string;
  sizeBytes: number;
  formatVersion: number | null;
  writerVersion: string | null;
  operationKind: string | null;
  startedAt: string | null;
  target: unknown;
  gateway: string | null;
  related: unknown;
  /** The recording's end outcome; `null` while it is incomplete. */
  outcome: string | null;
  /** Still being written by the server. */
  active: boolean;
  /** The last line is cut off (a crash or a full disk). */
  tornTail: boolean;
}

export interface DeleteOutcome {
  removed: number;
  /** Recordings being written right now; a later request deletes them. */
  keptActive: string[];
}

/** Which operation the report is about: a recording, or an id the operation has elsewhere. */
export type OperationRef =
  | { recordingId: string }
  | { relatedKey: string; relatedValue: string }
  | { selection: "none" };

export interface ProjectContext {
  serverIncarnation: string;
  projectIncarnation: number;
  revision: number;
}

export interface BundleReview {
  fingerprint: string;
  context: ProjectContext;
}

export interface BundleQuery {
  problemType: ProblemType;
  operation?: OperationRef;
  earlierRecordingIds?: string[];
  deviceId?: number;
  /** Ticked sections; omitted: the problem type's defaults. */
  sections?: SectionKind[];
  client?: Record<string, unknown>;
  description?: string;
  context?: ProjectContext;
  review?: BundleReview;
}

export type ReviewedBundleQuery = BundleQuery & { review: BundleReview };

export interface OfferView {
  kind: SectionKind;
  offer: Offer;
  selected: boolean;
  /** `null`: available. Otherwise the server's reason, in English. */
  unavailable: string | null;
}

export interface ContentMarkers {
  addresses: boolean;
  names: boolean;
  parameterValues: boolean;
  networkAddresses: boolean;
}

export interface SectionPreview {
  key: string;
  kind: SectionKind;
  label: string | null;
  bytes: number;
  entries: number;
  markers: ContentMarkers;
  notes: string[];
  files: string[];
}

export interface BundlePreview {
  review: BundleReview;
  problemType: ProblemType;
  recordingEnabled: boolean;
  operationRecordingId: string | null;
  deviceId: number | null;
  offers: OfferView[];
  sections: SectionPreview[];
  totalBytes: number;
  exceedsMailHint: boolean;
  fileName: string;
  mailDraft: string;
  recordings: RecordingSummary[];
}

export interface SavedBundle {
  blob: Blob;
  fileName: string;
}
