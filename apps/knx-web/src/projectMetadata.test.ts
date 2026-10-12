/** Modeled metadata stays exact and nullable, never a compatibility recomputation. */
import {expect,it} from "vitest";
import {admitProjectMetadata} from "./projectMetadata";
const view=()=>({server_incarnation:"fictional",project_incarnation:1,snapshot_revision:2,metadata:{view_version:1,scope:"normalizedProject",model_schema_version:12,project_id:"P-FICTION",name:"  <script>fictional()</script>  ",project_number:null,group_address_style:"ThreeLevel",completion:"Tested",last_modified:"2010-01-02T03:04:05+00:00",project_start:null,ets_schema_version:23,unlifted_group_address_dpt_declarations:3}});
it("admits exact modeled values without rewriting null into an empty number",()=>{
 expect(admitProjectMetadata(view())).toEqual(view());
 const empty={...view(),metadata:{...view().metadata,project_number:""}};expect(admitProjectMetadata(empty).metadata.project_number).toBe("");
});
it.each([null,{}, {...view(),metadata:{...view().metadata,view_version:2}}, {...view(),metadata:{...view().metadata,scope:"rawSource"}},
 {...view(),metadata:{...view().metadata,completion:"HardwareVerified"}}, {...view(),metadata:{...view().metadata,project_number:undefined}},
 {...view(),snapshot_revision:NaN},{...view(),metadata:{...view().metadata,unlifted_group_address_dpt_declarations:-1}}])("refuses unsupported/malformed or contradictory views",value=>{
 expect(()=>admitProjectMetadata(value)).toThrow();
});
it("charges cumulative UTF-8 bytes instead of only one field or code-unit length",()=>{
 const metadata={...view().metadata,project_id:"",name:"é".repeat(32768),project_number:null};
 expect(admitProjectMetadata({...view(),metadata}).metadata.name).toBe(metadata.name);
 expect(()=>admitProjectMetadata({...view(),metadata:{...metadata,project_number:"x"}})).toThrow();
});
it("rejects oversized modeled text rather than returning a truncated successful value",()=>{
 expect(()=>admitProjectMetadata({...view(),metadata:{...view().metadata,name:"x".repeat(65537)}})).toThrow();
});
