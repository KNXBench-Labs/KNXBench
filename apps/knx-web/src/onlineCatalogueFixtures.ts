/** Fictional online catalogue DTOs for component and browser tests; no provider or private data. */
import type { OnlineEntry, OnlineStatus } from "./onlineCatalogueApi";
export function onlineEntry(overrides: Partial<OnlineEntry> = {}): OnlineEntry {
 return {id:"fictional-1",manufacturer:"Fictional Controls",manufacturerId:"0042",name:"Fictional dimmer",orderNumbers:["FC-4"],fileName:"fictional.knxprod",fileType:"knxprod",size:1,sha256:"a".repeat(64),updated:"2026-01-01T00:00:00Z",
 programs:[{displayKey:"0042 0001 02",name:"Dimmer four channels",applicationNumber:"0001",version:"02",maskVersion:"0701",productNames:["Fictional dimmer 4x"]}],importable:true,downloaded:false,...overrides};
}
export function onlineStatus(overrides: Partial<OnlineStatus> = {}): OnlineStatus {
 return {source:"reko",label:"Reko",defaultBaseUrl:"https://catalogue.example",baseUrl:"https://catalogue.example/",baseUrlIsDefault:true,
 index:{entryCount:30,skippedEntries:1,version:"fictional-index",generatedAt:"2026-01-01T00:00:00Z",lastCheckedAt:"2026-01-02T00:00:00Z",fetchedAt:"2026-01-01T00:00:00Z"},indexProblem:null,
 credentials:{state:"empty",mode:null,user:null,problem:null},limit:null,syncRunning:false,download:{state:"idle",entryId:null,receivedBytes:0,totalBytes:null,error:null},...overrides};
}
