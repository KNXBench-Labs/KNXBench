/** Capture orchestration samples chronologically and refuses partial downloads. */
import { afterEach, expect, it, vi } from "vitest";
import * as capture from "./flowImageCapture";
const controller = () => new AbortController();
afterEach(() => { vi.useRealTimers(); });
it("requests exactly one bounded PNG raster from the selected viewport", async () => {
  const blob = new Blob(["adapter-only control"], { type: "image/png" });
  const raster = vi.fn(async ({ width, height }: { width: number; height: number }) => ({ rgba: new Uint8ClampedArray(width * height * 4), png: async () => blob }));
  const result = await capture.captureSelectedImage({ format: "png", width: 100, height: 50, signal: controller().signal, raster });
  expect(result).toBe(blob);
  expect(raster).toHaveBeenCalledOnce();
  expect(raster.mock.calls[0][0]).toEqual({ width: 960, height: 480 });
});

it("aborts a hung raster without a partial image after cancellation", async () => {
  const abort = controller(); const raster = vi.fn(() => new Promise<never>(() => {}));
  const job = capture.captureSelectedImage({ format: "png", width: 100, height: 50, signal: abort.signal, raster });
  const refused = expect(job).rejects.toThrow("cancelled"); abort.abort(); await refused;
});

it("times out a hung PNG encoder and releases every deadline timer", async () => {
  vi.useFakeTimers();
  const raster = async () => ({ rgba: new Uint8ClampedArray(4), png: () => new Promise<Blob>(() => {}) });
  const job = capture.captureSelectedImage({ format: "png", width: 100, height: 50, signal: controller().signal, raster });
  const refused = expect(job).rejects.toThrow("timeout");
  await vi.advanceTimersByTimeAsync(8000); await refused;
  expect(vi.getTimerCount()).toBe(0);
});

it("writes exactly 300 chronological samples for the selected maximum duration", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
  const times: number[] = []; let index = 0;
  const raster = async ({ width, height }: { width: number; height: number }) => {
    times.push(performance.now()); const rgba = new Uint8ClampedArray(width * height * 4);
    const value = index++ % 256;
    for (let i = 0; i < rgba.length; i += 4) { rgba[i] = (value >> 5) * 255 / 7; rgba[i+1] = ((value >> 2) & 7) * 255 / 7; rgba[i+2] = (value & 3) * 85; rgba[i+3] = 255; }
    return { rgba, png: async () => new Blob() };
  };
  const progress = vi.fn();
  const job = capture.captureSelectedImage({ format: "gif", seconds: 30, width: 1600, height: 900, signal: controller().signal, raster, progress });
  const received = expect(job).resolves.toHaveProperty("type", "image/gif");
  await vi.runAllTimersAsync(); await received;
  const blob = await job;
  expect(times).toEqual(Array.from({ length: 300 }, (_, i) => i * 100));
  expect(performance.now()).toBe(30000);
  expect(progress).toHaveBeenLastCalledWith(300, 300);
  expect(vi.getTimerCount()).toBe(0);
  if (process.env.FLOW_EXPORT_FIXTURES) {
    const { writeFileSync } = await import("node:fs");
    writeFileSync(`${process.env.FLOW_EXPORT_FIXTURES}/codec-maximum.gif`, new Uint8Array(await blob.arrayBuffer()));
  }
});

it("disposes the selected raster even when PNG encoding fails", async () => {
  const dispose = vi.fn();
  const raster = async () => ({ rgba: new Uint8ClampedArray(4), dispose, png: async (): Promise<Blob> => { throw new Error("encoding refused"); } });
  await expect(capture.captureSelectedImage({ format: "png", width: 100, height: 50, signal: controller().signal, raster })).rejects.toThrow("encoding refused");
  expect(dispose).toHaveBeenCalledOnce();
});
it("does not sample or return a partial GIF after cancellation between frames", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
  const abort = controller(); const dispose = vi.fn();
  const raster = vi.fn(async ({ width, height }: { width: number; height: number }) => ({ rgba: new Uint8ClampedArray(width * height * 4), dispose, png: async () => new Blob() }));
  const job = capture.captureSelectedImage({ format: "gif", seconds: 3, width: 100, height: 50, signal: abort.signal, raster, progress: completed => { if (completed === 2) abort.abort(); } });
  const refused = expect(job).rejects.toThrow("cancelled");
  await vi.runAllTimersAsync(); await refused;
  expect(raster).toHaveBeenCalledTimes(2); expect(dispose).toHaveBeenCalledTimes(2);
  expect(vi.getTimerCount()).toBe(0);
});

it("disposes a stale raster which finishes after the cancellation refusal", async () => {
  let finish!: (frame: capture.RasterFrame) => void; const abort = controller(); const dispose = vi.fn();
  const job = capture.captureSelectedImage({ format: "png", width: 100, height: 50, signal: abort.signal, raster: () => new Promise(resolve => { finish = resolve; }) });
  const refused = expect(job).rejects.toThrow("cancelled"); abort.abort(); await refused;
  finish({ rgba: new Uint8ClampedArray(4), png: async () => new Blob(), dispose });
  await Promise.resolve(); expect(dispose).toHaveBeenCalledOnce();
});

// The capture never skips frames: a frame that would start more than one
// interval late refuses the whole clip instead (port review, 2026-10-13).
function slowRaster(slowFrame: number, takesMs: number) {
  let index = 0;
  return vi.fn(async ({ width, height }: { width: number; height: number }) => {
    if (index++ === slowFrame) vi.advanceTimersByTime(takesMs);
    return { rgba: new Uint8ClampedArray(width * height * 4), png: async () => new Blob() };
  });
}

it("refuses a GIF clip, without a partial file, once a frame falls more than one interval behind", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
  const raster = slowRaster(1, 250);
  const job = capture.captureSelectedImage({ format: "gif", seconds: 3, width: 100, height: 50, signal: controller().signal, raster });
  const refused = expect(job).rejects.toThrow("tooSlow");
  await vi.runAllTimersAsync(); await refused;
  // Frame 1 ended at 350 ms; frame 2 was due at 200 ms, so it is never sampled.
  expect(raster).toHaveBeenCalledTimes(2);
  expect(vi.getTimerCount()).toBe(0);
});

it("keeps a clip whose frame is late by less than one interval, on the original schedule", async () => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "performance"] });
  const raster = slowRaster(1, 150);
  const job = capture.captureSelectedImage({ format: "gif", seconds: 3, width: 100, height: 50, signal: controller().signal, raster });
  const done = expect(job).resolves.toHaveProperty("type", "image/gif");
  await vi.runAllTimersAsync(); await done;
  expect(raster).toHaveBeenCalledTimes(30);
  expect(performance.now()).toBe(3000);
});
