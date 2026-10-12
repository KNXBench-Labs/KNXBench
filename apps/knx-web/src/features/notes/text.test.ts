/** Verifies literal textarea edits without normalizing untouched note text. */
import { describe, expect, it } from "vitest";
import { applyTextareaEdit, textareaText } from "./text";
describe("literal note text adapter",()=>{
 it("presents CRLF and CR as textarea line breaks without changing the model",()=>{
  expect(textareaText("A\r\nB\rC\nD")).toBe("A\nB\nC\nD");
  expect(applyTextareaEdit("A\r\nB\rC\nD","A\nB\nC\nD")).toBe("A\r\nB\rC\nD");
 });
 it("keeps every untouched mixed line ending when one character changes",()=>{
  expect(applyTextareaEdit("A\r\nB\rC\nD","A\nBX\nC\nD")).toBe("A\r\nBX\rC\nD");
 });
 it("uses the existing line ending for an explicitly inserted newline",()=>{
  expect(applyTextareaEdit("A\r\nB","A\nX\nB")).toBe("A\r\nX\r\nB");
  expect(applyTextareaEdit("A\rB","A\nX\nB")).toBe("A\rX\rB");
 });
 it("preserves whitespace, Unicode and complete surrogate pairs",()=>{
  expect(applyTextareaEdit(" Grüße🙂\r\n\tend "," Grüße🙃\n\tend ")).toBe(" Grüße🙃\r\n\tend ");
  expect(applyTextareaEdit("","\t \n🙂")).toBe("\t \n🙂");
 });
 it("does not interpret HTML or markdown-looking literal text",()=>{
  expect(applyTextareaEdit("<img src=\"https://untrusted.invalid\">\r\n**x**","<img src=\"https://untrusted.invalid\">\n**xx**")).toBe("<img src=\"https://untrusted.invalid\">\r\n**xx**");
 });
});
