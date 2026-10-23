/** One App-owned catalogue operation; capabilities and commands never leave this owner. */
import { catalogueJobsApi, CatalogueJobRefusal, type CatalogueJobsApi, type Owner } from "./catalogueJobsApi";
import { CatalogueProtocolError, parseCatalogueMessage, sameHandle, u64 } from "./catalogueJobsProtocol";
import type { ApplyCommand, Artifact, CancelCommand, CancelReply, Create, Page, Preview, Status } from "./catalogueJobsTypes";
export type CatalogueJobsSnapshot = {
 status: Status | null; origin: string | null; selecting: boolean; pending: boolean;
 error: string | null; uncertain: boolean; unavailable: boolean; applyUncertain: boolean;
 cancelOutcome: CancelReply["outcome"] | null; cancelQueued: boolean; catalogueRevision: bigint; committedOrigin: string | null;
 committed: boolean; previousUncertain: boolean; page: Page | null; pageBusy: boolean; legacyAvailable: boolean;
};
type Operation = {
 capability: string; create: Create; owner: Owner | null; file: File | null;
 uploadAttempted: boolean; apply: ApplyCommand | null; cancel: CancelCommand | null; notified: boolean;
};
export function catalogueRandomId(bytes = 16): string {
 const data = new Uint8Array(bytes); crypto.getRandomValues(data);
 return Array.from(data, n => n.toString(16).padStart(2,"0")).join("");
}
export class CatalogueJobsController {
 #op: Operation | null = null;
 #listeners = new Set<() => void>();
 #state: CatalogueJobsSnapshot = {status:null,origin:null,selecting:false,pending:false,error:null,uncertain:false,unavailable:false,applyUncertain:false,cancelOutcome:null,cancelQueued:false,catalogueRevision:0n,committedOrigin:null,committed:false,previousUncertain:false,page:null,pageBusy:false,legacyAvailable:false};
 #timer: ReturnType<typeof setTimeout> | null = null;
 #connected = false;
 #transport = new AbortController();
 #refreshing = false;
 #cancelling = false;
 #cancelIntent: CancelCommand | null = null;
 constructor(private readonly api: CatalogueJobsApi = catalogueJobsApi) {}
 getSnapshot = () => this.#state;
 subscribe = (listener: () => void) => { this.#listeners.add(listener); return () => { this.#listeners.delete(listener); }; };
 #patch(patch: Partial<CatalogueJobsSnapshot>) { this.#state = {...this.#state,...patch}; this.#listeners.forEach(fn => fn()); }
 connect = () => { this.#connected = true; if (this.#transport.signal.aborted) this.#transport = new AbortController(); if (this.#op?.owner) this.#op.owner.signal = this.#transport.signal; this.#schedule(); return () => { this.#connected = false; this.#transport.abort(); if (this.#timer !== null) clearTimeout(this.#timer); this.#timer = null; }; };
 #schedule() {
  if (!this.#connected || this.#timer !== null || !this.#op?.owner || this.#state.status?.state !== "running" || this.#state.unavailable || this.#state.error === "unauthenticated" || this.#state.error === "protocol") return;
  this.#timer = setTimeout(() => { this.#timer = null; void this.refresh(); },750);
 }
 #error(error: unknown) {
  const kind = error instanceof CatalogueJobRefusal ? error.kind : error instanceof CatalogueProtocolError ? "protocol" : "transport";
  this.#patch({error:kind,uncertain:this.#op !== null, unavailable:this.#state.unavailable || ["outcomeUnavailable","incarnationChanged"].includes(kind)});
 }
 #accept(status: Status, op: Operation) {
  if (this.#op !== op) return;
  // Also validate injected transports; tests cannot accidentally bypass status admission.
  const s = parseCatalogueMessage("Status",JSON.stringify(status));
  if (s.handle.serverIncarnation !== op.create.serverIncarnation || s.handle.admissionSerial !== op.create.admissionSerial || op.owner && !sameHandle(op.owner.handle,s.handle) || s.source.name !== op.create.sourceName || s.source.declaredBytes !== op.create.declaredBytes || s.profile.name !== op.create.profile) throw new CatalogueProtocolError();
  const old = this.#state.status;
  if (old) {
   if (u64(s.revision) < u64(old.revision)) return;
   if (s.revision === old.revision) { if (JSON.stringify(s) !== JSON.stringify(old)) throw new CatalogueProtocolError(); this.#patch({error:null,uncertain:this.#state.applyUncertain}); return; }
   if (old.state !== "running" && s.state !== old.state || old.source.sha256 !== null && old.source.sha256 !== s.source.sha256 || u64(s.source.receivedBytes) < u64(old.source.receivedBytes) || old.preview !== null && JSON.stringify(old.preview) !== JSON.stringify(s.preview) || old.result !== null && JSON.stringify(old.result) !== JSON.stringify(s.result) || old.apply.acceptedCommandId !== null && old.apply.acceptedCommandId !== s.apply.acceptedCommandId || old.gate !== "open" && old.gate !== s.gate || old.commit.outcome === "committed" && s.commit.outcome !== "committed" || old.phase === s.phase && old.state === s.state && old.lease.deadlineTickMs !== s.lease.deadlineTickMs) throw new CatalogueProtocolError();
  }
  if (s.apply.acceptedCommandId !== null && s.apply.acceptedCommandId !== op.apply?.commandId) throw new CatalogueProtocolError();
  op.owner ??= {handle:s.handle,capability:op.capability,signal:this.#transport.signal};
  const legacyAvailable = s.state === "failed" && s.failure?.kind === "legacyProductDatabase" && s.commit.outcome === "notCommitted" && s.cleanup.disposition === "complete" && op.file !== null;
  if (s.state !== "running" && !legacyAvailable) op.file = null;
  const committed = s.commit.outcome === "committed";
  const notify = committed && !op.notified; if (notify) op.notified = true;
  this.#patch({status:s, selecting:false,error:null,uncertain:false,legacyAvailable,applyUncertain:this.#state.applyUncertain && s.apply.acceptedCommandId === null && s.state === "running",committed:this.#state.committed || committed,
   ...(notify ? {catalogueRevision:this.#state.catalogueRevision+1n,committedOrigin:this.#state.origin} : {})});
 }
 async start(file: File, profile: "standard" | "large", acknowledged: boolean, origin: string | null) {
  if (this.#op || this.#state.selecting || profile === "large" && !acknowledged) return;
  const signal = this.#transport.signal;
  this.#patch({selecting:true,pending:true,error:null,origin});
  try {
   const offer = await this.api.admission(signal);
   if (signal.aborted) throw new Error("Transport closed");
   if (this.#cancelIntent) { this.#cancelIntent = null; this.#patch({cancelQueued:false}); return; }
   if (offer.admission !== "open") throw new CatalogueJobRefusal(offer.admission);
   const create: Create = {schemaVersion:1,policyVersion:"wp043-v1",serverIncarnation:offer.serverIncarnation,admissionSerial:offer.nextAdmissionSerial,sourceName:file.name,declaredBytes:BigInt(file.size).toString(),profile,allowLargePackage:profile === "large"};
   parseCatalogueMessage("Create",JSON.stringify(create));
   const capability = catalogueRandomId(32);
   const op: Operation = {capability,create,owner:null,file,uploadAttempted:false,apply:null,cancel:this.#cancelIntent,notified:false}; this.#op = op;
   this.#accept(await this.api.create(create,capability,signal),op);
   if (signal.aborted) throw new Error("Transport closed");
   if (op.cancel) { await this.cancel(); return; }
   if (op.owner && this.#state.status?.phase === "receiving") {
    op.uploadAttempted = true;
    // Start polling before awaiting upload: progress is server-written bytes, never browser sent bytes.
    this.#schedule();
    this.#accept(await this.api.upload(op.owner,file),op);
   }
  } catch (error) { this.#error(error); } finally { if (!this.#op) { this.#cancelIntent = null; this.#patch({cancelQueued:false}); } this.#patch({selecting:false,pending:false}); this.#schedule(); }
 }
 /** Explicit exact-create recovery finds the old handle only; it never starts an upload. */
 async recoverCreate() {
  const op = this.#op; if (!op || op.owner || this.#state.pending || this.#state.unavailable) return;
  this.#patch({pending:true});
  try { this.#accept(await this.api.create(op.create,op.capability,this.#transport.signal),op); op.file = null; if (op.cancel) await this.cancel(); } catch (error) { this.#error(error); } finally { this.#patch({pending:false}); this.#schedule(); }
 }
 async refresh() {
  const op = this.#op; if (!op?.owner || this.#refreshing || this.#state.unavailable) return;
  this.#refreshing = true;
  try { this.#accept(await this.api.status(op.owner),op); } catch (error) { this.#error(error); } finally { this.#refreshing = false; this.#schedule(); }
 }
 async apply(shown: Preview) {
  const op = this.#op; const s = this.#state.status;
  if (!op?.owner || op.apply || this.#state.pending || this.#state.unavailable || s?.phase !== "ready" || s.apply.state !== "available" || JSON.stringify(shown) !== JSON.stringify(s.preview)) return;
  // Assign before any await. A lost response spends this local command too; status-only recovery.
  op.apply = {schemaVersion:1,commandId:catalogueRandomId(),consentId:shown.consentId,previewArtifactId:shown.artifact.artifactId,previewPublicSha256:shown.artifact.publicSha256,confirmed:true};
  this.#patch({pending:true,applyUncertain:false,error:null});
  try { this.#accept(await this.api.apply(op.owner,op.apply),op); } catch (error) { this.#patch({applyUncertain:true}); this.#error(error); } finally { this.#patch({pending:false}); this.#schedule(); }
 }
 async cancel() {
  const op = this.#op;
  if (!op?.owner) {
   if (this.#state.selecting || op && this.#state.uncertain) { this.#cancelIntent ??= {schemaVersion:1,commandId:catalogueRandomId()}; if (op) op.cancel = this.#cancelIntent; this.#patch({cancelQueued:true}); }
   return;
  }
  if (this.#cancelling || this.#state.unavailable || this.#state.status?.state !== "running") return;
  op.cancel ??= {schemaVersion:1,commandId:catalogueRandomId()};
  this.#cancelling = true;
  this.#patch({pending:true});
  try {
   const reply = parseCatalogueMessage("CancelReply",JSON.stringify(await this.api.cancel(op.owner,op.cancel)));
   if (reply.commandId !== op.cancel.commandId) throw new CatalogueProtocolError();
   this.#accept(reply.job,op); this.#patch({cancelOutcome:reply.outcome,cancelQueued:false});
  } catch (error) { this.#error(error); } finally { this.#cancelling = false; this.#patch({pending:false}); this.#schedule(); }
 }
 async details(a: Artifact, offset = "0") {
  const op = this.#op; const s = this.#state.status;
  if (!op?.owner || this.#state.pageBusy || this.#state.unavailable || ![s?.preview?.artifact,s?.result?.artifact].some(v => v && JSON.stringify(v) === JSON.stringify(a))) return;
  this.#patch({pageBusy:true});
  try { const page = await this.api.details(op.owner,a,offset); if (this.#op === op) this.#patch({page,error:null}); } catch (error) { if (this.#op === op) this.#error(error); } finally { if (this.#op === op) this.#patch({pageBusy:false}); }
 }
 async dispose() {
  const op = this.#op;
  if (!op?.owner || this.#state.pending || this.#state.status?.state === "running" || this.#state.status?.cleanup.disposition !== "complete") return;
  this.#patch({pending:true});
  try { const reply = await this.api.dispose(op.owner); if (!sameHandle(reply.handle,op.owner.handle)) throw new CatalogueProtocolError(); this.#forget(false); } catch (error) { this.#error(error); } finally { this.#patch({pending:false}); }
 }
 /** Explicit compatibility handoff, once, after typed refusal and proved settlement. */
 takeLegacyFile(origin: string | null): File | null {
  if (!this.#state.legacyAvailable || origin !== this.#state.origin || !this.#op) return null;
  const file = this.#op.file; this.#op.file = null; this.#patch({legacyAvailable:false}); return file;
 }
 /** Only explicit acknowledgement forgets unavailable history. This does NOT assert server stop. */
 forgetUnavailable() { if (!this.#state.pending && (this.#state.unavailable || this.#state.uncertain && !this.#op?.owner || !this.#op)) this.#forget(true); }
 #forget(uncertain: boolean) {
  this.#op = null; this.#cancelIntent = null; if (this.#timer !== null) clearTimeout(this.#timer); this.#timer = null;
  this.#patch({status:null,origin:null,selecting:false,error:null,uncertain:false,unavailable:false,applyUncertain:false,cancelOutcome:null,cancelQueued:false,committed:false,page:null,pageBusy:false,legacyAvailable:false,previousUncertain:this.#state.previousUncertain || uncertain});
 }
}
