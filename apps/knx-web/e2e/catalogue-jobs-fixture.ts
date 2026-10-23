/** Synthetic job transport for built-App UI contracts, never native backend acceptance. */
import { expect, type Route } from "@playwright/test";
import { createHash } from "node:crypto";
import { bindPage, parseCatalogueMessage } from "../src/catalogueJobsProtocol";
import type { ApplyCommand, Artifact, Create, Handle, Page, PublicRecord, Status } from "../src/catalogueJobsTypes";

const BASE = "/api/catalog/update/jobs";
const INCARNATION = "a".repeat(32);
const digest = (bytes: Buffer | string) => createHash("sha256").update(bytes).digest("hex");
interface Options {
  records?: (kind: Artifact["kind"]) => PublicRecord[];
  backup?: string;
  stale?: () => boolean;
  legacy?: boolean;
  onApplied?: () => void;
}

/** Exact responses and headers are validated before fulfilment, not repaired to pass parsing. */
export function catalogueJobsFixture(options: Options = {}) {
  const creates: Create[] = [];
  const uploads: Buffer[] = [];
  const applies: ApplyCommand[] = [];
  const disposes: Handle[] = [];
  let current: Status | null = null;
  let capability: string | null = null;
  const artifacts = new Map<string, { artifact: Artifact; records: PublicRecord[] }>();
  const reply = (route: Route, body: unknown, status = 200) => route.fulfill({
    status, headers: { "Content-Type": "application/json", "Cache-Control": "no-store" }, body: JSON.stringify(body),
  });
  const statusReply = (route: Route, status = 200) => {
    expect(current).not.toBeNull();
    parseCatalogueMessage("Status", JSON.stringify(current));
    return reply(route, current, status);
  };
  function artifact(kind: Artifact["kind"]): Artifact {
    const records = options.records!(kind);
    const bytes = records.map(row => JSON.stringify(row) + "\n").join("");
    const withheld = records.filter(row => row.withheld.length > 0);
    const value: Artifact = {
      artifactId: digest(`${creates.length}-${kind}`).slice(0, 32), kind, publicSha256: digest(bytes),
      publicBytes: String(Buffer.byteLength(bytes)), records: String(records.length),
      withheldFields: String(records.reduce((n, row) => n + row.withheld.length, 0)),
      withheldRecords: String(withheld.length),
      withheldOccurrences: withheld.reduce((n, row) => n + ("occurrences" in row ? BigInt(row.occurrences) : 1n), 0n).toString(),
      projection: "redactedCompleteV1",
    };
    // Validate the exact full page, including ordinal, summary, path and redaction contracts.
    const page: Page = { schemaVersion: 1, handle: current!.handle, artifactId: value.artifactId,
      publicSha256: value.publicSha256, offset: "0", totalRecords: value.records, records, nextOffset: null };
    parseCatalogueMessage("Page", JSON.stringify(page));
    bindPage(page, current!.handle, value, "0", 200);
    artifacts.set(value.artifactId, { artifact: value, records });
    return value;
  }
  function answer(route: Route): Promise<void> | null {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    const method = request.method();
    if (path === `${BASE}/admission` && method === "GET") {
      const offer = { schemaVersion: 1, policyVersion: "wp043-v1", serverIncarnation: INCARNATION,
        nextAdmissionSerial: String(creates.length + 1), serverTickMs: "1", admission: current ? "busy" : "open" };
      parseCatalogueMessage("Admission", JSON.stringify(offer));
      return reply(route, offer);
    }
    if (path === BASE && method === "POST") {
      expect(current).toBeNull();
      const command = parseCatalogueMessage("Create", request.postData()!);
      expect(command.serverIncarnation).toBe(INCARNATION);
      expect(command.admissionSerial).toBe(String(creates.length + 1));
      capability = request.headers()["x-knx-catalogue-capability"];
      expect(capability).toMatch(/^[0-9a-f]{64}$/);
      creates.push(command);
      current = {
        schemaVersion: 1, policyVersion: "wp043-v1", handle: { serverIncarnation: INCARNATION,
          admissionSerial: command.admissionSerial, jobId: digest(`job-${command.admissionSerial}`).slice(0, 32) }, revision: "1",
        source: { name: command.sourceName, declaredBytes: command.declaredBytes, receivedBytes: "0", sha256: null },
        profile: { name: command.profile, maxMemberBytes: "1000000000",
          maxExpandedBytes: command.profile === "large" ? "17179869184" : "4294967296" },
        state: "running", phase: "receiving", gate: "open", stopCause: null, progress: null,
        lease: { kind: "receiving", deadlineTickMs: "90000" }, apply: { state: "unavailable", acceptedCommandId: null },
        preview: null, result: null, failure: null, commit: { outcome: "pending", evidence: "notAttempted" },
        cleanup: { disposition: "pending", failureKind: null }, backup: { state: "none", name: null, identity: null, publication: "none" },
      };
      return statusReply(route, 201);
    }
    if (!current || !path.startsWith(`${BASE}/${current.handle.jobId}`)) return null;
    // Never handle another job or an unexpected action as a generic successful API request.
    const suffix = path.slice(`${BASE}/${current.handle.jobId}`.length);
    if (![["", "GET"], ["", "DELETE"], ["/upload", "PUT"], ["/apply", "POST"], ["/details", "GET"]]
      .some(([expectedPath, expectedMethod]) => suffix === expectedPath && method === expectedMethod)) return null;
    expect(request.headers()["x-knx-catalogue-capability"]).toBe(capability);
    expect(request.headers()["x-knx-catalogue-incarnation"]).toBe(current.handle.serverIncarnation);
    expect(request.headers()["x-knx-catalogue-serial"]).toBe(current.handle.admissionSerial);
    if (suffix === "/upload") {
      expect(current.phase).toBe("receiving");
      expect(request.headers()["content-type"]).toBe("application/octet-stream");
      const bytes = request.postDataBuffer()!;
      expect(String(bytes.length)).toBe(current.source.declaredBytes);
      uploads.push(bytes);
      current = { ...current, revision: "2", source: { ...current.source, receivedBytes: String(bytes.length), sha256: digest(bytes) } };
      if (options.legacy) {
        current = { ...current, state: "failed", phase: null, lease: { kind: "retention", deadlineTickMs: "95000" },
          apply: { state: "spent", acceptedCommandId: null }, failure: { kind: "legacyProductDatabase", context: null },
          commit: { outcome: "notCommitted", evidence: "noLiveMutation" }, cleanup: { disposition: "complete", failureKind: null } };
      } else {
        const publicArtifact = artifact("preview");
        const summary = artifacts.get(publicArtifact.artifactId)!.records[0];
        if (summary.kind !== "previewSummary") throw new Error("Fixture requires a preview summary");
        const { ordinal: _ordinal, kind: _kind, withheld: _withheld, ...counts } = summary;
        current = { ...current, phase: "ready", lease: { kind: "ready", deadlineTickMs: "94000" },
          apply: { state: "available", acceptedCommandId: null },
          preview: { consentId: digest(`consent-${creates.length}`).slice(0, 32), artifact: publicArtifact, ...counts } };
      }
      return statusReply(route, 202);
    }
    if (suffix === "/apply") {
      expect(current.phase).toBe("ready");
      const command = parseCatalogueMessage("ApplyCommand", request.postData()!);
      expect(command).toEqual({ schemaVersion: 1, commandId: expect.stringMatching(/^[0-9a-f]{32}$/),
        consentId: current.preview!.consentId, previewArtifactId: current.preview!.artifact.artifactId,
        previewPublicSha256: current.preview!.artifact.publicSha256, confirmed: true });
      applies.push(command);
      current = { ...current, revision: "3", phase: null, lease: { kind: "retention", deadlineTickMs: "98000" },
        apply: { state: "spent", acceptedCommandId: command.commandId }, cleanup: { disposition: "complete", failureKind: null } };
      if (options.stale?.()) {
        current = { ...current, state: "stale", failure: { kind: "catalogueUpdateStale", context: null },
          commit: { outcome: "notCommitted", evidence: "noLiveMutation" } };
      } else {
        current = { ...current, state: "succeeded", gate: "commitClaimed", commit: { outcome: "committed", evidence: "commitReturnedOk" },
          backup: { state: "retained", name: options.backup!, identity: "2".repeat(32), publication: "fileAndDirectorySyncAcknowledged" },
          result: { artifact: artifact("result"), cataloguePageBytes: "3145728", nextJobInputLimitExceeded: false } };
        options.onApplied?.();
      }
      return statusReply(route, 202);
    }
    if (suffix === "/details") {
      const query = parseCatalogueMessage("DetailQuery", JSON.stringify({ artifactId: url.searchParams.get("artifactId"),
        publicSha256: url.searchParams.get("publicSha256"), offset: url.searchParams.get("offset"), limit: Number(url.searchParams.get("limit")) }));
      const saved = artifacts.get(query.artifactId)!;
      expect(saved).toBeDefined();
      const selected = [current.preview?.artifact, current.result?.artifact].find(value => value?.artifactId === query.artifactId);
      expect(selected).toEqual(saved.artifact);
      expect(query.publicSha256).toBe(saved.artifact.publicSha256);
      expect(query.limit).toBe(200);
      const offset = Number(query.offset);
      const records = saved.records.slice(offset, offset + query.limit);
      const end = offset + records.length;
      const page: Page = { schemaVersion: 1, handle: current.handle, artifactId: saved.artifact.artifactId,
        publicSha256: saved.artifact.publicSha256, offset: query.offset, totalRecords: saved.artifact.records,
        records, nextOffset: end < saved.records.length ? String(end) : null };
      parseCatalogueMessage("Page", JSON.stringify(page));
      bindPage(page, current.handle, saved.artifact, query.offset, query.limit);
      return reply(route, page);
    }
    if (method === "DELETE") {
      expect(current.state).not.toBe("running");
      expect(current.cleanup.disposition).toBe("complete");
      const body = { schemaVersion: 1, handle: current.handle, disposition: "disposed" };
      parseCatalogueMessage("DisposeReply", JSON.stringify(body));
      disposes.push(current.handle); current = null; capability = null;
      return reply(route, body);
    }
    return statusReply(route);
  }
  return { answer, creates, uploads, applies, disposes, status: () => current };
}
