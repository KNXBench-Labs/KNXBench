/** User-selected GIF durations obey the hard thirty-second resource boundary. */
import { expect, it } from "vitest";
import * as limits from "./flowImageExportLimits";
it("offers a complete 30-second, 300-frame clip with a declared smaller raster", () => {
  expect(limits.gifProfile(30, 1600, 900)).toEqual({ seconds: 30, frames: 300, width: 320, height: 180, deadlineMs: 35000 });
});

it.each([0, -1, 31, Infinity, NaN, 1.5])("refuses invalid duration %s instead of silently clamping it", seconds => {
  expect(() => limits.gifProfile(seconds, 1600, 900)).toThrow("invalidInput");
});
it("preserves the default three-second viewport aspect ratio", () => {
  expect(limits.gifProfile(limits.DEFAULT_GIF_SECONDS, 100, 50)).toEqual({ seconds: 3, frames: 30, width: 640, height: 320, deadlineMs: 8000 });
});
it("keeps every allowed duration's worst literal output inside the declared byte cap", () => {
  for (let seconds = 1; seconds <= 30; seconds++) {
    const profile = limits.gifProfile(seconds, 1600, 900);
    const pixels = profile.width * profile.height;
    const codes = Math.ceil((pixels + Math.ceil(pixels / 200) + 1) * 9 / 8);
    expect(782 + profile.frames * (20 + codes + Math.ceil(codes / 255))).toBeLessThan(limits.MAX_EXPORT_BYTES);
  }
});
