/** Explicit resource policy for selected-viewport images, never a bus recording. */
export const PNG_BOUNDS = { width: 960, height: 540 } as const;
export const GIF_BOUNDS = { width: 640, height: 360 } as const;
export const MAX_GIF_SECONDS = 30;
export const DEFAULT_GIF_SECONDS = 3;
export const GIF_FRAMES = MAX_GIF_SECONDS * 10;
export const GIF_FRAME_MS = 100;
export const EXPORT_DEADLINE_MS = 8000;
export const MAX_EXPORT_BYTES = 32 * 1024 * 1024;
export const MAX_EXPORT_SVG_BYTES = 1024 * 1024;
export const MAX_EXPORT_ELEMENTS = 4000;
export type ImageExportFailure = "invalidInput" | "tooLarge" | "unsafeSvg" | "cancelled" | "timeout" | "tooSlow" | "rasterFailed";
export class ImageExportError extends Error {
  constructor(readonly kind: ImageExportFailure) { super(kind); this.name = "ImageExportError"; }
}
export function imageDimensions(width: number, height: number, bounds: { width: number; height: number }) {
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) throw new ImageExportError("invalidInput");
  const scale = Math.min(bounds.width / width, bounds.height / height);
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
}

export function gifProfile(seconds: number, width: number, height: number) {
  if (!Number.isInteger(seconds) || seconds < 1 || seconds > MAX_GIF_SECONDS) throw new ImageExportError("invalidInput");
  const bounds = seconds <= 5 ? GIF_BOUNDS : seconds <= 15 ? { width: 480, height: 270 } : { width: 320, height: 180 };
  return { seconds, frames: seconds * 10, ...imageDimensions(width, height, bounds), deadlineMs: seconds * 1000 + 5000 };
}
