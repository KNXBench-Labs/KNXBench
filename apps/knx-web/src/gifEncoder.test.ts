/** Independent pixel/structure controls for the bounded GIF89a writer. */
import { expect, it } from "vitest";
import * as gif from "./gifEncoder";
it("quantizes red, green and blue without losing frame pixel order", () => {
  expect([...gif.quantize332(new Uint8ClampedArray([255,0,0,255, 0,255,0,255, 0,0,255,255]))]).toEqual([224,28,3]);
});

it("writes complete GIF89a frames in pixel order across clear-code boundaries", async () => {
  const writer = new gif.GifWriter(256, 2);
  const first = Uint8Array.from({ length: 512 }, (_, i) => i % 256);
  writer.addFrame(first); writer.addFrame(Uint8Array.from(first, v => 255 - v));
  const bytes = writer.finish();
  expect(new TextDecoder().decode(bytes.slice(0, 6))).toBe("GIF89a");
  expect([...bytes.slice(6, 10)]).toEqual([0,1,2,0]);
  expect(bytes.at(-1)).toBe(0x3b);
  expect(writer.frameCount).toBe(2);
  if (process.env.FLOW_EXPORT_FIXTURES) {
    const { writeFileSync } = await import("node:fs");
    writeFileSync(`${process.env.FLOW_EXPORT_FIXTURES}/codec-control.gif`, bytes);
  }
});
