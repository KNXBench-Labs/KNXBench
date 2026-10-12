/** Browser bytes downloads use inert local blobs and a fixed safe filename. */
// @vitest-environment happy-dom
import {afterEach,expect,it,vi} from "vitest";
import {downloadLocalBlob} from "./localJsonDownload";
afterEach(()=>{vi.restoreAllMocks();vi.unstubAllGlobals();});
it("saves the supplied inert blob without rendering its contents",()=>{
 const create=vi.fn().mockReturnValue("blob:fictional");const revoke=vi.fn();
 vi.stubGlobal("URL",{createObjectURL:create,revokeObjectURL:revoke});
 const click=vi.spyOn(HTMLAnchorElement.prototype,"click").mockImplementation(()=>{});
 const schedule=vi.spyOn(window,"setTimeout").mockReturnValue(0 as unknown as ReturnType<typeof window.setTimeout>);
 const blob=new Blob(["<script>not executed</script>"],{type:"application/octet-stream"});
 downloadLocalBlob(blob,"retained-payload.bin");expect(create).toHaveBeenCalledWith(blob);
 expect(click).toHaveBeenCalledOnce();expect(document.querySelector("a")).toBeNull();
 expect(document.querySelector("script")).toBeNull();expect(schedule).toHaveBeenCalled();
});
