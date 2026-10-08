/** Records lossless real-browser frames and optimises focused manual GIFs with ffmpeg. */
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test, type Page } from "@playwright/test";

/** All actions run against the real isolated server; only the camera is post-production. */
export async function recordManualGif(page: Page, name: string, actions: () => Promise<void>): Promise<void> {
  const dir = test.info().outputPath(`frames-${name}`);
  mkdirSync(dir, { recursive: true });
  const frames: { path: string; time: number }[] = [];
  const cdp = await page.context().newCDPSession(page);
  cdp.on("Page.screencastFrame", (frame) => {
    const path = `${dir}/${String(frames.length).padStart(5, "0")}.png`;
    writeFileSync(path, Buffer.from(frame.data, "base64"));
    frames.push({ path, time: frame.metadata.timestamp! });
    void cdp.send("Page.screencastFrameAck", { sessionId: frame.sessionId }).catch(() => {});
  });
  await cdp.send("Page.startScreencast", { format: "png", maxWidth: 1440, maxHeight: 900 });
  try {
    await page.waitForTimeout(500);
    await actions();
    await page.waitForTimeout(1200);
  } finally {
    await cdp.send("Page.stopScreencast");
    await cdp.detach();
  }
  if (frames.length < 2) throw new Error(`No usable frames for ${name}`);
  const list = frames.map((frame, i) => {
    const duration = i + 1 < frames.length ? Math.max(0.02, frames[i + 1].time - frame.time) : 1.2;
    return `file '${frame.path}'\nduration ${duration.toFixed(4)}\n`;
  }).join("") + `file '${frames[frames.length - 1].path}'\n`;
  const input = `${dir}/frames.txt`;
  writeFileSync(input, list);
  const output = resolve(process.env.MANUAL_GIF_DIR
    ?? fileURLToPath(new URL("../../../docs/assets/workflows", import.meta.url)));
  mkdirSync(output, { recursive: true });
  execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", input,
    "-filter_complex", "fps=10,scale=960:600:flags=lanczos,split[a][b];[a]palettegen=max_colors=80:stats_mode=diff[p];[b][p]paletteuse=dither=none:diff_mode=rectangle",
    "-loop", "0", `${output}/${name}.gif`], { timeout: 120_000 });
  console.log(`Recorded ${name}.gif from ${frames.length} real browser frames`);
}
