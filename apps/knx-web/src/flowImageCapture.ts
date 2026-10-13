/** Local bounded capture orchestration; raster adapter owns selected graph pixels. */
export interface RasterFrame { rgba: Uint8ClampedArray; png: () => Promise<Blob>; dispose?: () => void }
export interface CaptureRequest {
  format: "png" | "gif"; width: number; height: number; seconds?: number;
  signal: AbortSignal;
  raster: (dimensions: { width: number; height: number }, signal: AbortSignal) => Promise<RasterFrame>;
  progress?: (completed: number, total: number) => void;
}
import { DEFAULT_GIF_SECONDS, EXPORT_DEADLINE_MS, GIF_FRAME_MS, ImageExportError, MAX_EXPORT_BYTES, PNG_BOUNDS, gifProfile, imageDimensions } from "./flowImageExportLimits";
import { GifWriter, quantize332 } from "./gifEncoder";
function checkAbort(signal: AbortSignal): void {
  if (signal.aborted) throw new ImageExportError(signal.reason === "timeout" ? "timeout" : "cancelled");
}
function abortable<T>(work: Promise<T>, signal: AbortSignal, discard?: (value: T) => void): Promise<T> {
  return new Promise((resolve, reject) => {
    const abort = () => { cleanup(); reject(new ImageExportError(signal.reason === "timeout" ? "timeout" : "cancelled")); };
    const cleanup = () => signal.removeEventListener("abort", abort);
    signal.addEventListener("abort", abort, { once: true });
    work.then(value => { cleanup(); if (signal.aborted) { discard?.(value); reject(new ImageExportError(signal.reason === "timeout" ? "timeout" : "cancelled")); } else resolve(value); }, error => { cleanup(); reject(error); });
    if (signal.aborted) abort();
  });
}
function wait(milliseconds: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const abort = () => { clearTimeout(timer); reject(new ImageExportError(signal.reason === "timeout" ? "timeout" : "cancelled")); };
    const timer = setTimeout(() => { signal.removeEventListener("abort", abort); resolve(); }, Math.max(0, milliseconds));
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) { signal.removeEventListener("abort", abort); abort(); }
  });
}
export async function captureSelectedImage(request: CaptureRequest): Promise<Blob> {
  if (request.format !== "png" && request.format !== "gif") throw new ImageExportError("invalidInput");
  const profile = request.format === "gif" ? gifProfile(request.seconds ?? DEFAULT_GIF_SECONDS, request.width, request.height) : null;
  const dimensions = profile ? { width: profile.width, height: profile.height } : imageDimensions(request.width, request.height, PNG_BOUNDS);
  checkAbort(request.signal);
  const abort = new AbortController();
  const forward = () => abort.abort(request.signal.reason);
  request.signal.addEventListener("abort", forward, { once: true });
  const deadline = setTimeout(() => abort.abort("timeout"), profile?.deadlineMs ?? EXPORT_DEADLINE_MS);
  try {
    if (!profile) {
      const frame = await abortable(request.raster(dimensions, abort.signal), abort.signal, frame => frame.dispose?.());
      try {
        checkAbort(abort.signal);
        const blob = await abortable(frame.png(), abort.signal);
        if (blob.type !== "image/png") throw new ImageExportError("rasterFailed");
        if (blob.size > MAX_EXPORT_BYTES) throw new ImageExportError("tooLarge");
        request.progress?.(1, 1); return blob;
      } finally { frame.dispose?.(); }
    }
    const writer = new GifWriter(dimensions.width, dimensions.height);
    const started = performance.now();
    for (let index = 0; index < profile.frames; index++) {
      const target = started + index * GIF_FRAME_MS;
      checkAbort(abort.signal);
      if (performance.now() - target > GIF_FRAME_MS) throw new ImageExportError("tooSlow");
      await wait(target - performance.now(), abort.signal); checkAbort(abort.signal);
      const frame = await abortable(request.raster(dimensions, abort.signal), abort.signal, frame => frame.dispose?.());
      try {
        checkAbort(abort.signal);
        if (frame.rgba.length !== dimensions.width * dimensions.height * 4) throw new ImageExportError("invalidInput");
        writer.addFrame(quantize332(frame.rgba)); request.progress?.(index + 1, profile.frames);
      } finally { frame.dispose?.(); }
    }
    await wait(started + profile.seconds * 1000 - performance.now(), abort.signal);
    checkAbort(abort.signal);
    return new Blob([writer.finish()], { type: "image/gif" });
  } finally {
    clearTimeout(deadline); request.signal.removeEventListener("abort", forward);
  }
}
