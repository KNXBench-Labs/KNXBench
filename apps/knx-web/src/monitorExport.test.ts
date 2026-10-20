/** Sparse-export reports never turn missing evidence into measured zero or false. */
import { describe, expect, it } from "vitest";
import { admitMonitorExportPreview, monitorExportRequest } from "./monitorExport";
import {tree, request, preview} from "./monitorExport.test-fixtures";
describe("monitoring export admission", () => {
  it("captures full camelCase identity and selected options from the actual ProjectTree", () => {
    expect(monitorExportRequest(tree(),7,"")).toEqual(request);
    expect(monitorExportRequest(tree(),7,"de")).toEqual({...request,language:"de"});
    expect(monitorExportRequest(tree(),null,"")).toBeNull();
    expect(monitorExportRequest({...tree(),snapshot_revision:undefined},7,"")).toBeNull();
    expect(monitorExportRequest(tree(),8,"")).toBeNull();
  });
  it("admits the actual InstallationId u8 boundary and refuses a larger selector", () => {
    const atBoundary={...tree(),installations:[{...tree().installations[0],id:255}]};
    expect(monitorExportRequest(atBoundary,255,"")?.installation).toBe(255);
    expect(monitorExportRequest({...atBoundary,installations:[{...atBoundary.installations[0],id:256}]},256,"")).toBeNull();
  });
  it("admits the complete snake_case report without dropping omissions", () => {
    expect(admitMonitorExportPreview(preview(),request)).toEqual(preview());
  });
  it.each([
    ["version", (v: any) => v.report.format_version=2],
    ["profile", (v: any) => v.report.profile="full"],
    ["producer", (v: any) => v.report.producer="other"],
    ["hash", (v: any) => v.report.project_sha256="not-a-hash"],
    ["bundle hash", (v: any) => v.bundleSha256=""],
    ["missing count", (v: any) => delete v.report.counts.spaces],
    ["negative count", (v: any) => v.report.counts.spaces.omitted=-1],
    ["inconsistent count", (v: any) => v.report.counts.devices.omitted=0],
    ["unsafe count", (v: any) => v.report.counts.spaces.selected=Number.MAX_SAFE_INTEGER+1],
    ["missing issues", (v: any) => delete v.report.issues],
    ["issue detail", (v: any) => v.report.issues[0].detail=false],
    ["missing excluded data", (v: any) => delete v.report.excluded_data],
    ["unknown field", (v: any) => v.report.verified=true],
    ["different installation", (v: any) => v.report.installation=8],
    ["different revision", (v: any) => v.binding.projectRevision=3],
    ["different project", (v: any) => v.binding.projectIncarnation=2],
    ["different server", (v: any) => v.binding.serverIncarnation="other"],
  ])("refuses %s instead of normalizing it", (_label,change) => {
    const value=preview();change(value);expect(()=>admitMonitorExportPreview(value,request)).toThrow();
  });
});
