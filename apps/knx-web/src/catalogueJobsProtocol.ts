/** Bounded, duplicate-aware WP04.3 parsing against the exact checked-in schemas. */
import wire from "../../../docs/contracts/catalogue-jobs/wp043-protocol.schema.json";
import records from "../../../docs/contracts/catalogue-jobs/wp043-artifact-records.schema.json";
import policy from "../../../docs/contracts/catalogue-jobs/wp043-protocol-policy.json";
import type * as T from "./catalogueJobsTypes";

export class CatalogueProtocolError extends Error {
  constructor() { super("Invalid catalogue job protocol response"); }
}
const fail = (): never => { throw new CatalogueProtocolError(); };
export const utf8Bytes = (text: string) => new TextEncoder().encode(text).byteLength;
export function u64(value: unknown): bigint {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]{0,19})$/.test(value)) return fail();
  const n = BigInt(value);
  if (n > 18446744073709551615n) return fail();
  return n;
}

/** Decode object keys before checking uniqueness; JSON.parse alone loses duplicates. */
function strictJson(text: string): unknown {
  let i = 0;
  const space = () => { while (/[\x20\x09\x0a\x0d]/.test(text[i] ?? "!") && i < text.length) i++; };
  const string = (): string => {
    const start = i++;
    while (i < text.length) {
      if (text[i] === "\\") { i += 2; continue; }
      if (text[i++] === '"') {
        const value: unknown = JSON.parse(text.slice(start, i));
        if (typeof value !== "string" || /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(value)) return fail();
        return value;
      }
    }
    return fail();
  };
  function value(depth: number, key?: string, recordElement = false): void {
    space();
    const start = i;
    const finish = () => { if (recordElement && utf8Bytes(text.slice(start, i)) > 65536) fail(); };
    const ch = text[i];
    if (ch === "{" || ch === "[") {
      if (depth >= 8) fail();
      const object = ch === "{"; const end = object ? "}" : "]"; i++; space();
      const keys = new Set<string>();
      if (text[i] === end) { i++; finish(); return; }
      for (;;) {
        let memberKey: string | undefined;
        if (object) {
          if (text[i] !== '"') fail();
          const key = string(); if (keys.has(key)) fail(); keys.add(key); memberKey = key; space();
          if (text[i++] !== ":") fail();
        }
        value(depth + 1, memberKey, !object && depth === 1 && key === "records"); space();
        if (text[i] === end) { i++; finish(); return; }
        if (text[i++] !== ",") fail(); space();
      }
    }
    if (ch === '"') { string(); return; }
    const match = /^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/.exec(text.slice(i));
    if (!match) fail(); else i += match[0].length;
  }
  if (text.charCodeAt(0) === 0xfeff) fail();
  try { value(0); space(); if (i !== text.length) fail(); return JSON.parse(text) as unknown; } catch { return fail(); }
}

type Shape = { [key: string]: unknown };
const wireDefs = wire.$defs as unknown as Record<string, Shape>;
const recordDefs = records.$defs as unknown as Record<string, Shape>;
/** Only the vocabulary actually used in these two protocol schemas, not a general schema service. */
function matches(s: Shape, v: unknown, defs: Record<string, Shape>): boolean {
  if (typeof s.$ref === "string") {
    const external = s.$ref.startsWith("wp043-artifact-records.schema.json");
    const d = external ? recordDefs : defs;
    if (!matches(d[s.$ref.split("/").at(-1)!], v, d)) return false;
  }
  if ("const" in s && v !== s.const) return false;
  if (Array.isArray(s.enum) && !s.enum.includes(v)) return false;
  if (s.type === "null" && v !== null) return false;
  if (s.type === "boolean" && typeof v !== "boolean") return false;
  if (s.type === "integer" && (typeof v !== "number" || !Number.isSafeInteger(v))) return false;
  if (typeof v === "number" && (!Number.isFinite(v) || typeof s.minimum === "number" && v < s.minimum || typeof s.maximum === "number" && v > s.maximum)) return false;
  if (s.type === "string" && typeof v !== "string") return false;
  if (typeof v === "string") {
    if (typeof s.maxLength === "number" && [...v].length > s.maxLength || typeof s["x-maxUtf8Bytes"] === "number" && utf8Bytes(v) > s["x-maxUtf8Bytes"] || typeof s.pattern === "string" && !new RegExp(s.pattern).test(v)) return false;
    if (typeof s["x-maximumDecimal"] === "string") { try { if (u64(v) > BigInt(s["x-maximumDecimal"])) return false; } catch { return false; } }
  }
  if (s.type === "array" && !Array.isArray(v)) return false;
  if (Array.isArray(v)) {
    if (typeof s.maxItems === "number" && v.length > s.maxItems) return false;
    if (s.items && !v.every(item => matches(s.items as Shape, item, defs))) return false;
  }
  if (s.type === "object" && (v === null || typeof v !== "object" || Array.isArray(v))) return false;
  if (v !== null && typeof v === "object" && !Array.isArray(v)) {
    const obj = v as Record<string, unknown>; const properties = (s.properties ?? {}) as Record<string, Shape>;
    if (Array.isArray(s.required) && !s.required.every(k => typeof k === "string" && Object.hasOwn(obj, k))) return false;
    if (s.additionalProperties === false && Object.keys(obj).some(k => !Object.hasOwn(properties, k))) return false;
    if (!Object.entries(properties).every(([k, spec]) => !Object.hasOwn(obj, k) || matches(spec, obj[k], defs))) return false;
  }
  if (s.not && matches(s.not as Shape, v, defs)) return false;
  if (Array.isArray(s.allOf) && !s.allOf.every(spec => matches(spec as Shape, v, defs))) return false;
  if (Array.isArray(s.anyOf) && !s.anyOf.some(spec => matches(spec as Shape, v, defs))) return false;
  if (Array.isArray(s.oneOf) && s.oneOf.filter(spec => matches(spec as Shape, v, defs)).length !== 1) return false;
  if (s.if && !matches((matches(s.if as Shape, v, defs) ? s.then : s.else ?? {}) as Shape, v, defs)) return false;
  if (typeof s["x-maxSerializedUtf8Bytes"] === "number" && utf8Bytes(JSON.stringify(v)) > s["x-maxSerializedUtf8Bytes"]) return false;
  return true;
}
export function safeCatalogueName(name: string): boolean {
  return utf8Bytes(name) >= 1 && utf8Bytes(name) <= 512 && !/[\\/:\p{Cc}\p{Cf}]/u.test(name) && name !== "." && name !== ".." && !/^[\x09-\x0d ]|[\x09-\x0d ]$/.test(name);
}
function artifact(a: T.Artifact): void {
  if (u64(a.publicBytes) > 67108864n || u64(a.records) === 0n || u64(a.records) > 100000n || u64(a.withheldRecords) > u64(a.records) || u64(a.withheldFields) < u64(a.withheldRecords)) fail();
}
export function validateStatus(s: T.Status): void {
  if (u64(s.revision) < 1n || u64(s.handle.admissionSerial) < 1n || u64(s.handle.admissionSerial) === 18446744073709551615n || !safeCatalogueName(s.source.name) || u64(s.source.receivedBytes) > u64(s.source.declaredBytes) || u64(s.source.declaredBytes) > 1000000000n || s.source.sha256 !== null && s.source.receivedBytes !== s.source.declaredBytes) fail();
  if (s.profile.maxExpandedBytes !== (s.profile.name === "large" ? "17179869184" : "4294967296")) fail();
  if ((s.gate === "cancelAccepted") !== (s.stopCause !== null)) fail();
  if ((s.apply.state === "accepted" && s.apply.acceptedCommandId === null) || ["available", "unavailable"].includes(s.apply.state) && s.apply.acceptedCommandId !== null) fail();
  const p = s.progress;
  if (p) {
    if (u64(p.pass) === 0n || p.total !== null && u64(p.completed) > u64(p.total)) fail();
    const units: Record<string, string[]> = { receiving: ["bytes"], hashing: ["bytes"], decoding: ["bytes"], archiveScan: ["entries"], xmlParsing: ["events"], snapshotCopy: ["pages"], backupCopy: ["pages"], reporting: ["records"], cleanup: ["records"], binding: ["bytes", "events", "records"], rehearsal: ["bytes", "events", "records"], install: ["bytes", "events", "records"] };
    if (!units[p.stage].includes(p.unit)) fail();
  }
  const key = s.state === "running" ? s.phase : s.state;
  const rule = (policy.stateRules as Record<string, Record<string, unknown>>)[key ?? ""];
  if (!rule) fail();
  const allowed = (expected: unknown, actual: unknown) => Array.isArray(expected) ? expected.includes(actual) : expected === actual;
  for (const [field, actual] of [["gate", s.gate], ["apply", s.apply.state], ["lease", s.lease.kind], ["commit", s.commit.outcome], ["backup", s.backup.state], ["cleanup", s.cleanup.disposition]] as const) if (rule[field] !== undefined && !allowed(rule[field], actual)) fail();
  if (rule.sealed === true && s.source.sha256 === null || rule.sealed === false && s.source.sha256 !== null) fail();
  for (const field of ["preview", "result", "failure"] as const) {
    if (rule[field] === "required" && s[field] === null || rule[field] === "null" && s[field] !== null) fail();
  }
  if (Array.isArray(rule.failure) && !rule.failure.includes(s.failure?.kind)) fail();
  if (s.state === "running" && s.phase !== "settling" && s.failure !== null) fail();
  if (s.state === "running" && s.phase === "settling" && p && (p.stage !== "cleanup" || p.unit !== "records")) fail();
  if (s.preview) artifact(s.preview.artifact);
  if (s.result) {
    artifact(s.result.artifact);
    if (s.commit.outcome !== "committed" || !s.preview || s.result.nextJobInputLimitExceeded !== (u64(s.result.cataloguePageBytes) > 8589934592n) || u64(s.result.cataloguePageBytes) > 68719476736n) fail();
  }
  if (s.commit.outcome === "committed" && (s.gate !== "commitClaimed" || s.apply.acceptedCommandId === null || s.preview === null || s.result === null || s.source.sha256 === null)) fail();
  if (s.state === "expired" && s.stopCause === "readyExpired" && s.preview === null) fail();
  if (s.backup.name !== null && !safeCatalogueName(s.backup.name)) fail();
  if (s.state === "cancelled" && !["userCancel", "serverStop"].includes(s.stopCause ?? "")) fail();
  if (s.state === "cancelled" && s.failure?.kind !== (s.stopCause === "userCancel" ? "cancelled" : "serverStop")) fail();
  if (s.state === "expired" && (s.failure?.kind !== s.stopCause || !["readyExpired", "receivingIdleTimeout", "receivingTimeout", "workingTimeout"].includes(s.stopCause ?? ""))) fail();
}

function archivePath(value: string): boolean {
  return utf8Bytes(value) <= 8192 && value.length > 0 && !/[\\:\p{Cc}\p{Cf}]/u.test(value) && value.split("/").every(part => part !== "" && part !== "." && part !== "..");
}
function publicRecord(r: T.PublicRecord): void {
  const obj = r as unknown as Record<string, unknown>;
  for (const key of ["path", "archivePath", "indexPath", "memberPath"]) if (typeof obj[key] === "string" && !archivePath(obj[key])) fail();
  const xmlName = (value: string) => utf8Bytes(value) <= 512 && /^[A-Za-z_][A-Za-z0-9_.:-]*$/.test(value);
  for (const key of ["xmlPath", "xpath"]) if (typeof obj[key] === "string" && (utf8Bytes(obj[key]) > 8192 || obj[key] !== "/" && (!(obj[key] as string).startsWith("/") || !(obj[key] as string).slice(1).split("/").every(xmlName)))) fail();
  if (r.kind === "unknownConstruct" && !xmlName(r.name)) fail();
  if (r.kind === "idConflict" && (![r.table, r.id].every(v => utf8Bytes(v) <= 512 && /^[A-Za-z0-9_][A-Za-z0-9_.-]*$/.test(v)) || u64(r.occurrence) === 0n)) fail();
  const fields: Partial<Record<T.PublicRecord["kind"], string[]>> = { program: ["name"], unknownConstruct: ["sample"], diagnostic: ["detail"], provenance: ["alias"], baggageDeclaration: ["declarationId", "name", "targetPath", "installOnImport", "timeInfo", "fileVersion", "detail"] };
  const allowed = fields[r.kind] ?? [];
  let last = -1;
  for (const w of r.withheld) { const at = allowed.indexOf(w.field); if (at <= last) fail(); last = at; }
  if (["diagnostic", "provenance"].includes(r.kind) && r.withheld.length !== 1) fail();
  if (r.kind === "packageReport" && (r.facts === "unavailable") !== (r.unknownOccurrences === null)) fail();
  if (r.kind === "baggageSummary" && (r.status === "unavailable" ? r.declarations !== null || r.payloads !== null : r.declarations === null || r.payloads === null)) fail();
  if (r.kind === "baggageDeclaration" && (r.resolution === "resolved") !== (r.memberPath !== null)) fail();
  if (r.kind === "baggagePayload" && [r.nested.entries, r.nested.declaredExpandedSize, r.nested.encryptedEntries, r.nested.archiveNamedEntries].some(v => (r.nested.state === "read") === (v === null))) fail();
}
type Messages = { Admission: T.Admission; Status: T.Status; Create: T.Create; ApplyCommand: T.ApplyCommand; CancelCommand: T.CancelCommand; CancelReply: T.CancelReply; DisposeReply: T.DisposeReply; Error: T.Error; DetailQuery: T.DetailQuery; Page: T.Page };
export function parseCatalogueMessage<K extends keyof Messages>(kind: K, text: string): Messages[K] {
  const v = strictJson(text);
  if (!matches(wireDefs[kind], v, wireDefs)) fail();
  if (kind === "Status") validateStatus(v as T.Status);
  if (kind === "CancelReply") {
    const reply = v as T.CancelReply; validateStatus(reply.job);
    if (reply.outcome === "alreadyTerminal" ? reply.job.state === "running" : reply.job.state !== "running" || (["requested", "alreadyRequested"].includes(reply.outcome) ? reply.job.gate !== "cancelAccepted" || reply.job.phase !== "settling" : reply.job.gate !== "commitClaimed")) fail();
  }
  if (kind === "Create") { const c = v as T.Create; if (!safeCatalogueName(c.sourceName) || u64(c.declaredBytes) > 1000000000n || c.allowLargePackage !== (c.profile === "large") || u64(c.admissionSerial) === 0n || u64(c.admissionSerial) === 18446744073709551615n) fail(); }
  if (kind === "Admission") { const a = v as T.Admission; if (u64(a.nextAdmissionSerial) === 0n || (a.admission === "exhausted") !== (u64(a.nextAdmissionSerial) === 18446744073709551615n)) fail(); }
  if (kind === "Page") { const page = v as T.Page; page.records.forEach(publicRecord); }
  return v as Messages[K];
}
export const sameHandle = (a: T.Handle, b: T.Handle) => a.serverIncarnation === b.serverIncarnation && a.admissionSerial === b.admissionSerial && a.jobId === b.jobId;
export function bindPage(page: T.Page, handle: T.Handle, a: T.Artifact, offset: string, limit: number): void {
  const end = u64(offset) + BigInt(page.records.length); const total = u64(a.records);
  if (!sameHandle(page.handle, handle) || page.artifactId !== a.artifactId || page.publicSha256 !== a.publicSha256 || page.totalRecords !== a.records || page.offset !== offset || page.records.length > limit || end > total || u64(offset) < total && page.records.length === 0 || page.nextOffset !== (end < total ? end.toString() : null)) fail();
  page.records.forEach((r, i) => {
    if (u64(r.ordinal) !== u64(offset) + BigInt(i)) fail();
    if (r.ordinal === "0" && r.kind !== (a.kind === "preview" ? "previewSummary" : "resultSummary") || r.ordinal !== "0" && ["previewSummary", "resultSummary"].includes(r.kind)) fail();
  });
}
