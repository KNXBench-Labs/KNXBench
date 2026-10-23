/** Strict WP04.3 wire grammar and semantic refusals (no server acceptance claim). */
import { describe, expect, it } from "vitest";
import { status } from "./catalogueJobsFixtures";
import { bindPage, parseCatalogueMessage, u64 } from "./catalogueJobsProtocol";

const admission = { schemaVersion: 1, policyVersion: "wp043-v1", serverIncarnation: "a".repeat(32), nextAdmissionSerial: "9007199254740993", serverTickMs: "1", admission: "open" };
describe("catalogue job protocol", () => {
  it("keeps U64 values exact and refuses every noncanonical numeric form", () => {
    expect(u64("18446744073709551615")).toBe(18446744073709551615n);
    for (const value of ["18446744073709551616", "01", "+1", " 1", "1\n", "1e3", "１", 1]) expect(() => u64(value)).toThrow();
    expect(parseCatalogueMessage("Admission", JSON.stringify(admission))).toEqual(admission);
  });
  it("rejects duplicate decoded keys, BOM, deep nesting, unknown and absent keys", () => {
    for (const raw of [JSON.stringify(admission).replace('"schemaVersion":1', '"schemaVersion":1,"schema\\u0056ersion":1'), '\ufeff' + JSON.stringify(admission), JSON.stringify({ ...admission, secret: "no" }), JSON.stringify({ ...admission, admission: undefined }), '['.repeat(9) + '0' + ']'.repeat(9)]) expect(() => parseCatalogueMessage("Admission", raw)).toThrow();
  });
  it("accepts coherent Ready and refuses impossible cross-field and nested shapes", () => {
    const ready = status("ready", "2");
    expect(parseCatalogueMessage("Status", JSON.stringify(ready))).toEqual(ready);
    const edits = [
      (s: ReturnType<typeof status>) => {s.source.receivedBytes = "2";},
      (s: ReturnType<typeof status>) => {s.handle.jobId += "\n";},
      (s: ReturnType<typeof status>) => {s.profile.maxExpandedBytes = "4294967296";},
      (s: ReturnType<typeof status>) => {s.commit.evidence = "commitReturnedOk";},
      (s: ReturnType<typeof status>) => {s.gate = "cancelAccepted";},
      (s: ReturnType<typeof status>) => {s.cleanup.disposition = "quarantined";},
      (s: ReturnType<typeof status>) => {s.apply.acceptedCommandId = "a".repeat(32);},
      (s: ReturnType<typeof status>) => {s.progress = {stage:"snapshotCopy",pass:"1",unit:"bytes",completed:"1",total:"0"};},
    ];
    for (const edit of edits) {const s = structuredClone(ready);edit(s);expect(() => parseCatalogueMessage("Status",JSON.stringify(s))).toThrow();}
    expect(() => parseCatalogueMessage("Status",JSON.stringify(ready).replace('"gate":"open"','"gate":"open","gate":"open"'))).toThrow();
  });
  it("binds complete page offsets, exact descriptors, ordinal coverage and safe redaction", () => {
    const s=status("ready","2"); const a=s.preview!.artifact;
    const row={ordinal:"0",kind:"previewSummary",packageAlreadyInstalled:false,wrapper:false,packages:"1",programs:"0",productsAdded:"0",hardware2ProgramsAdded:"0",idConflicts:"0",growthCliffPossible:true,withheld:[]};
    const data={schemaVersion:1,handle:s.handle,artifactId:a.artifactId,publicSha256:a.publicSha256,offset:"0",totalRecords:"1",records:[row],nextOffset:null};
    const page=parseCatalogueMessage("Page",JSON.stringify(data)); expect(() => bindPage(page,s.handle,a,"0",200)).not.toThrow();
    for(const changed of [{...page,nextOffset:"1"},{...page,records:[]},{...page,publicSha256:"a".repeat(64)},{...page,records:[{...page.records[0],ordinal:"1"}]}]) expect(() => bindPage(changed,s.handle,a,"0",200)).toThrow();
    const dangerous={ordinal:"0",kind:"provenance",sourceKey:"0",packageKey:"0",sha256:"a".repeat(64),alias:"/home/alice/source",withheld:[]};
    expect(() => parseCatalogueMessage("Page",JSON.stringify({...data,records:[dangerous]}))).toThrow();
    const baggage={ordinal:"0",kind:"baggageDeclaration",packageKey:"0",indexPath:"M-1/Baggages.xml",declarationOrdinal:"0",declarationId:null,name:null,targetPath:null,installOnImport:null,timeInfo:null,fileVersion:null,detail:null,resolution:"missing",memberPath:"payload.bin",withheld:[]};
    expect(() => parseCatalogueMessage("Page",JSON.stringify({...data,records:[baggage]}))).toThrow();
  });
  it("enforces numeric bounds rather than accepting shape-compatible DTOs", () => {
    expect(() => parseCatalogueMessage("DetailQuery", JSON.stringify({ artifactId: "b".repeat(32), publicSha256: "c".repeat(64), offset: "0", limit: 201 }))).toThrow();
    expect(() => parseCatalogueMessage("Create", JSON.stringify({ schemaVersion: 1, policyVersion: "wp043-v1", serverIncarnation: "a".repeat(32), admissionSerial: "1", sourceName: "../x.knxprod", declaredBytes: "1", profile: "large", allowLargePackage: false }))).toThrow();
  });
});
