/** Strict catalogue-specific transport; raw bytes are required before JSON.parse. */
import { noteRefusal } from "./api";
import policy from "../../../docs/contracts/catalogue-jobs/wp043-protocol-policy.json" with { type: "json" };
import { bindPage, CatalogueProtocolError, parseCatalogueMessage, u64 } from "./catalogueJobsProtocol";
import type { Admission, ApplyCommand, Artifact, CancelCommand, CancelReply, Create, DisposeReply, Handle, Page, Status } from "./catalogueJobsTypes";
export type Owner = { handle: Handle; capability: string; signal?: AbortSignal };
export interface CatalogueJobsApi {
 admission(signal?: AbortSignal): Promise<Admission>;
 create(command: Create, capability: string, signal?: AbortSignal): Promise<Status>;
 upload(owner: Owner, file: File): Promise<Status>;
 status(owner: Owner): Promise<Status>;
 apply(owner: Owner, command: ApplyCommand): Promise<Status>;
 cancel(owner: Owner, command: CancelCommand): Promise<CancelReply>;
 details(owner: Owner, artifact: Artifact, offset: string): Promise<Page>;
 dispose(owner: Owner): Promise<DisposeReply>;
}
export class CatalogueJobRefusal extends Error {
 constructor(readonly kind: string) { super("Catalogue job request refused"); }
}
const base = "/api/catalog/update/jobs";
const id = (owner: Owner) => { headers(owner); return `${base}/${owner.handle.jobId}`; };
function headers(owner: Owner): Record<string, string> {
 if (owner.capability.length !== 64 || !/^[0-9a-f]{64}$/.test(owner.capability) || !/^[0-9a-f]{32}$/.test(owner.handle.serverIncarnation) || owner.handle.serverIncarnation.length !== 32 || !/^[0-9a-f]{32}$/.test(owner.handle.jobId) || owner.handle.jobId.length !== 32 || u64(owner.handle.admissionSerial) === 0n) throw new CatalogueProtocolError();
 return { "X-KNX-Catalogue-Capability": owner.capability, "X-KNX-Catalogue-Incarnation": owner.handle.serverIncarnation, "X-KNX-Catalogue-Serial": owner.handle.admissionSerial };
}
/** api.request currently parses JSON before returning; that would destroy duplicate-key evidence.
 * Keep its existing auth notification boundary, but read these bounded responses directly. */
async function raw(path: string, init: RequestInit, cap = 16384, expected = [200]): Promise<string> {
 const response = await fetch(path, { ...init, cache: "no-store", credentials: "same-origin", redirect: "error" });
 if (response.status === 401) { noteRefusal(path, 401); await response.body?.cancel(); throw new CatalogueJobRefusal("unauthenticated"); }
 if (!/^application\/json(?:;\s*charset=utf-8)?$/i.test(response.headers.get("Content-Type") ?? "") || response.headers.get("Cache-Control") !== "no-store" || ![null, "identity"].includes(response.headers.get("Content-Encoding"))) { await response.body?.cancel(); throw new CatalogueProtocolError(); }
 if (response.ok && !expected.includes(response.status)) { await response.body?.cancel(); throw new CatalogueProtocolError(); }
 const length = response.headers.get("Content-Length");
 if (length !== null && u64(length) > BigInt(response.ok ? cap : 16384)) { await response.body?.cancel(); throw new CatalogueProtocolError(); }
 const reader = response.body?.getReader(); if (!reader) throw new CatalogueProtocolError();
 const decoder = new TextDecoder("utf-8", {fatal: true, ignoreBOM: true}); let text = ""; let bytes = 0;
 try {
  for (;;) { const {value,done} = await reader.read(); if (done) break; bytes += value.byteLength; if (bytes > (response.ok ? cap : 16384)) throw new CatalogueProtocolError(); text += decoder.decode(value,{stream:true}); }
  text += decoder.decode();
 } catch { await reader.cancel().catch(() => {}); throw new CatalogueProtocolError(); } finally { reader.releaseLock(); }
 if (length !== null && u64(length) !== BigInt(bytes)) throw new CatalogueProtocolError();
 if (!response.ok) { const error = parseCatalogueMessage("Error", text); if (policy.errors[error.kind].httpStatus !== response.status) throw new CatalogueProtocolError(); throw new CatalogueJobRefusal(error.kind); }
 return text;
}
const json = (method: string, body: unknown, extra: Record<string,string> = {}): RequestInit => ({ method, headers: {"Content-Type":"application/json", ...extra}, body: JSON.stringify(body) });
export const catalogueJobsApi: CatalogueJobsApi = {
 async admission(signal) { return parseCatalogueMessage("Admission", await raw(`${base}/admission`,{method:"GET",signal})); },
 async create(command, capability, signal) { if (capability.length !== 64 || !/^[0-9a-f]{64}$/.test(capability)) throw new CatalogueProtocolError(); parseCatalogueMessage("Create", JSON.stringify(command)); return parseCatalogueMessage("Status", await raw(base, {...json("POST",command,{"X-KNX-Catalogue-Capability":capability}),signal},16384,[200,201])); },
 async upload(owner, file) { return parseCatalogueMessage("Status", await raw(`${id(owner)}/upload`,{method:"PUT",signal:owner.signal,headers:{...headers(owner),"Content-Type":"application/octet-stream"},body:file},16384,[202])); },
 async status(owner) { return parseCatalogueMessage("Status", await raw(id(owner),{method:"GET",signal:owner.signal,headers:headers(owner)})); },
 async apply(owner, command) { parseCatalogueMessage("ApplyCommand",JSON.stringify(command)); return parseCatalogueMessage("Status", await raw(`${id(owner)}/apply`,{...json("POST",command,headers(owner)),signal:owner.signal},16384,[200,202])); },
 async cancel(owner, command) { parseCatalogueMessage("CancelCommand",JSON.stringify(command)); const reply = parseCatalogueMessage("CancelReply", await raw(`${id(owner)}/cancel`,{...json("POST",command,headers(owner)),signal:owner.signal})); if (reply.commandId !== command.commandId) throw new CatalogueProtocolError(); return reply; },
 async details(owner, artifact, offset) {
  const query = {artifactId:artifact.artifactId,publicSha256:artifact.publicSha256,offset,limit:200}; parseCatalogueMessage("DetailQuery",JSON.stringify(query));
  const page = parseCatalogueMessage("Page", await raw(`${id(owner)}/details?${new URLSearchParams({...query,limit:"200"})}`,{method:"GET",signal:owner.signal,headers:headers(owner)},262144)); bindPage(page,owner.handle,artifact,offset,200); return page;
 },
 async dispose(owner) { return parseCatalogueMessage("DisposeReply", await raw(id(owner),{method:"DELETE",signal:owner.signal,headers:headers(owner)})); },
};
