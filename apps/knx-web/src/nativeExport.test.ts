/** Native local exports never silently use an unreviewed or anonymous snapshot. */
import {expect,it} from "vitest";
import type {ProjectTree} from "./bindings/ProjectTree";
import {nativeExportMatches,nativeExportPath} from "./nativeExport";
const tree=():ProjectTree=>({schema_version:12,errors:0,warnings:0,can_undo:false,can_redo:false,is_modified:false,group_address_style:"ThreeLevel",installations:[],server_incarnation:"fictional & server",project_incarnation:1,snapshot_revision:2});
it("constructs only a full literal server/load/revision bound native URL",()=>{
 expect(nativeExportMatches(tree(),tree())).toBe(true);
 const url=new URL(nativeExportPath(tree()),"http://127.0.0.1");expect(url.pathname).toBe("/api/project/download");
 expect(url.searchParams.get("serverIncarnation")).toBe(tree().server_incarnation);expect(url.searchParams.get("projectIncarnation")).toBe("1");expect(url.searchParams.get("snapshotRevision")).toBe("2");
});
it.each([null,{...tree(),server_incarnation:"other"},{...tree(),project_incarnation:2},{...tree(),snapshot_revision:3}])("rejects a changed reviewed source",current=>{
 expect(nativeExportMatches(tree(),current)).toBe(false);
});
it.each([{...tree(),server_incarnation:undefined},{...tree(),project_incarnation:-1},{...tree(),snapshot_revision:NaN}])("refuses missing or malformed identities rather than legacy fallback",reviewed=>{
 expect(nativeExportMatches(reviewed,reviewed)).toBe(false);expect(()=>nativeExportPath(reviewed)).toThrow();
});
