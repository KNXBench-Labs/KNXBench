/** Feature-local copy has matching language keys and informal German address. */
import { describe,expect,it } from "vitest";
import { noteCopy } from "./copy";
describe("notes copy",()=>{
 it("keeps EN/DE keys complete and values nonempty",()=>{expect(Object.keys(noteCopy.en).sort()).toEqual(Object.keys(noteCopy.de).sort());for(const value of [...Object.values(noteCopy.en),...Object.values(noteCopy.de)])expect(value.length).toBeGreaterThan(0);});
 it("uses du rather than formal Sie/Ihr in every German feature string",()=>{for(const value of Object.values(noteCopy.de))expect(value).not.toMatch(/\b(?:Sie|Ihr|Ihren)\b/u);});
});
