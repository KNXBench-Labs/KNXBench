/** Deterministic bounded GIF89a export (CompuServe specification, Appendix F). */
import { GIF_BOUNDS, GIF_FRAMES, MAX_EXPORT_BYTES, ImageExportError } from "./flowImageExportLimits";

export function quantize332(rgba: Uint8ClampedArray): Uint8Array {
  if (rgba.length % 4 !== 0) throw new ImageExportError("invalidInput");
  const result = new Uint8Array(rgba.length / 4);
  for (let i = 0; i < result.length; i++) result[i] = (rgba[i * 4] & 224) | ((rgba[i * 4 + 1] >> 3) & 28) | (rgba[i * 4 + 2] >> 6);
  return result;
}

/** Fixed nine-bit literals; clear before the decoder dictionary could grow to ten bits. */
function literalCodes(pixels: Uint8Array): Uint8Array<ArrayBuffer> {
  const packed = new Uint8Array(Math.ceil((pixels.length + Math.ceil(pixels.length / 200) + 1) * 9 / 8));
  let used = 0, value = 0, bits = 0;
  const emit = (code: number) => {
    value |= code << bits; bits += 9;
    while (bits >= 8) { packed[used++] = value & 255; value >>>= 8; bits -= 8; }
  };
  for (let i = 0; i < pixels.length; i++) { if (i % 200 === 0) emit(256); emit(pixels[i]); }
  emit(257); if (bits > 0) packed[used++] = value & 255;
  return packed.subarray(0, used);
}

/** Full-frame GIF89a, deterministic RGB332 table, 100ms/frame, no looping extension. */
export class GifWriter {
  private parts: Uint8Array<ArrayBuffer>[] = [];
  private bytes = 0;
  private closed = false;
  private frames = 0;
  constructor(readonly width: number, readonly height: number) {
    if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) throw new ImageExportError("invalidInput");
    if (width > GIF_BOUNDS.width || height > GIF_BOUNDS.height) throw new ImageExportError("tooLarge");
    const header = new Uint8Array(13 + 768);
    header.set(new TextEncoder().encode("GIF89a"));
    header.set([width & 255, width >> 8, height & 255, height >> 8, 0xf7, 0, 0], 6);
    for (let i = 0; i < 256; i++) header.set([Math.round((i >> 5) * 255 / 7), Math.round(((i >> 2) & 7) * 255 / 7), (i & 3) * 85], 13 + i * 3);
    this.append(header);
  }
  get frameCount(): number { return this.frames; }
  get byteLength(): number { return this.bytes; }
  private append(data: Uint8Array<ArrayBuffer>) {
    if (this.bytes + data.length + 1 > MAX_EXPORT_BYTES) throw new ImageExportError("tooLarge");
    this.parts.push(data); this.bytes += data.length;
  }
  addFrame(pixels: Uint8Array): void {
    if (this.closed || pixels.length !== this.width * this.height) throw new ImageExportError("invalidInput");
    if (this.frames >= GIF_FRAMES) throw new ImageExportError("tooLarge");
    const codes = literalCodes(pixels);
    const frame = new Uint8Array(19 + codes.length + Math.ceil(codes.length / 255) + 1);
    frame.set([0x21, 0xf9, 4, 4, 10, 0, 0, 0, 0x2c, 0, 0, 0, 0, this.width & 255, this.width >> 8, this.height & 255, this.height >> 8, 0, 8]);
    let used = 19;
    for (let i = 0; i < codes.length; i += 255) {
      const block = codes.subarray(i, i + 255); frame[used++] = block.length;
      frame.set(block, used); used += block.length;
    }
    frame[used] = 0; this.append(frame); this.frames++;
  }
  finish(): Uint8Array<ArrayBuffer> {
    if (this.closed || this.frames === 0) throw new ImageExportError("invalidInput");
    this.closed = true;
    const result = new Uint8Array(this.bytes + 1); let at = 0;
    for (const part of this.parts) { result.set(part, at); at += part.length; }
    result[at] = 0x3b; this.parts = [];
    return result;
  }
}
