/** Explicit local export of the chosen graph viewport, with bounded capture. */
import { useEffect, useId, useRef, useState, type RefObject } from "react";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import { rasterizeSelectedSvg, serializeSelectedSvg, type ViewSelection } from "./flowImageExport";
import { captureSelectedImage } from "./flowImageCapture";
import type { MessageKey } from "./messages/en";
import { DEFAULT_GIF_SECONDS, ImageExportError, MAX_GIF_SECONDS, PNG_BOUNDS, gifProfile, imageDimensions } from "./flowImageExportLimits";
interface Props {
  svgRef: RefObject<SVGSVGElement | null>; selection: ViewSelection; sourceKey: string;
  active: boolean; initialFormat: "png" | "gif"; onClose: () => void;
}
export default function FlowImageExportDialog(props: Props) {
  const t = useTranslate(); const titleId = useId();
  const [format, setFormat] = useState(props.initialFormat);
  const [seconds, setSeconds] = useState(String(DEFAULT_GIF_SECONDS));
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false); const [completed, setCompleted] = useState(0);
  const [downloaded, setDownloaded] = useState(false);
  const [failure, setFailure] = useState<MessageKey | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const initialSource = useRef(props.sourceKey);
  const [invalidated, setInvalidated] = useState(false);
  const current = useRef({ sourceKey: props.sourceKey, active: props.active });
  current.current = { sourceKey: props.sourceKey, active: props.active };
  const mounted = useRef(true); const job = useRef<AbortController | null>(null);
  const delivery = useRef<{ url: string; timer: ReturnType<typeof setTimeout> } | null>(null);
  const sourceValid = !invalidated && props.active && props.sourceKey === initialSource.current;
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; job.current?.abort(); };
  }, []);
  useEffect(() => {
    if (!sourceValid) { setInvalidated(true); job.current?.abort(); setFailure("flow.export.sourceChanged"); }
  }, [sourceValid]);
  useEffect(() => {
    const hidden = () => { if (document.hidden) job.current?.abort(); };
    document.addEventListener("visibilitychange", hidden);
    return () => document.removeEventListener("visibilitychange", hidden);
  }, []);
  useEffect(() => {
    const svg = props.svgRef.current; if (!svg || !sourceValid) return;
    let url: string | null = null;
    try {
      url = URL.createObjectURL(new Blob([serializeSelectedSvg(svg, props.selection)], { type: "image/svg+xml;charset=utf-8" }));
      setPreview(url);
    } catch (error) { setFailure(error instanceof ImageExportError ? `flow.export.${error.kind}` : "flow.export.rasterFailed"); }
    return () => { if (url) URL.revokeObjectURL(url); };
  }, [props.selection, props.svgRef, sourceValid]);
  const duration = Number(seconds);
  const valid = Number.isInteger(duration) && duration >= 1 && duration <= MAX_GIF_SECONDS;
  const dimensions = format === "gif" && valid ? gifProfile(duration, props.selection.width, props.selection.height) : imageDimensions(props.selection.width, props.selection.height, PNG_BOUNDS);
  const total = format === "gif" && valid ? duration * 10 : 1;
  const stillCurrent = () => mounted.current && !invalidated && current.current.active && current.current.sourceKey === initialSource.current;
  async function start() {
    if (job.current || !consent || !sourceValid || (format === "gif" && !valid) || document.hidden) return;
    const abort = new AbortController(); job.current = abort;
    setBusy(true); setCompleted(0); setFailure(null); setDownloaded(false);
    try {
      const blob = await captureSelectedImage({ format, seconds: duration, width: props.selection.width, height: props.selection.height, signal: abort.signal,
        raster: (dimensions, signal) => {
          const svg = props.svgRef.current;
          if (!svg || !stillCurrent()) throw new ImageExportError("cancelled");
          return rasterizeSelectedSvg(svg, props.selection, dimensions, signal);
        }, progress: (done) => { if (stillCurrent()) setCompleted(done); },
      });
      if (abort.signal.aborted || !stillCurrent()) throw new ImageExportError("cancelled");
      if (delivery.current) { clearTimeout(delivery.current.timer); URL.revokeObjectURL(delivery.current.url); delivery.current = null; }
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a"); anchor.href = url; anchor.download = `knxbench-flow.${format}`;
      document.body.append(anchor);
      try { anchor.click(); } finally {
        anchor.remove();
        delivery.current = { url, timer: setTimeout(() => { URL.revokeObjectURL(url); if (delivery.current?.url === url) delivery.current = null; }, 10000) };
      }
      if (stillCurrent()) setDownloaded(true);
    } catch (error) {
      if (mounted.current) setFailure(!stillCurrent() ? "flow.export.sourceChanged" : error instanceof ImageExportError ? `flow.export.${error.kind}` : "flow.export.rasterFailed");
    } finally { job.current = null; if (mounted.current) setBusy(false); }
  }
  return <Overlay labelledBy={titleId} className="flow-export-dialog" onClose={props.onClose}>
    <h2 id={titleId}>{t("flow.export.title")}</h2>
    <p>{t("flow.export.selection")}</p>
    {preview && sourceValid && <figure className="flow-export-preview"><img src={preview} style={{ backgroundColor: props.selection.background }} alt={t("flow.export.preview")} /><figcaption>{t("flow.export.preview")}</figcaption></figure>}
    <label>{t("flow.export.format")}<select disabled={busy} value={format} onChange={event => setFormat(event.target.value as "png" | "gif")}>
      <option value="png">{t("flow.export.png")}</option><option value="gif">{t("flow.export.gif")}</option>
    </select></label>
    {format === "gif" && <label>{t("flow.export.duration")}<input disabled={busy} type="number" min="1" max={MAX_GIF_SECONDS} step="1" value={seconds} aria-invalid={!valid} onChange={event => setSeconds(event.target.value)} />
      {!valid && <span className="field-error">{t("flow.export.durationInvalid")}</span>}
    </label>}
    <p className="flow-export-resolution">{t("flow.export.resolution", dimensions)}</p>
    {format === "gif" && <p>{t("flow.export.limits")}</p>}
    <label className="flow-export-consent"><input disabled={busy} type="checkbox" checked={consent} onChange={event => setConsent(event.target.checked)} /><span>{t("flow.export.privacy")}</span></label>
    {failure && <p className="field-error" role="alert">{t(failure)}</p>}
    <p role="status"><span className={busy ? "task-status-text" : undefined}>{busy ? t("flow.export.working") : downloaded ? t("flow.export.downloadStarted") : ""}</span></p>
    {busy && <progress value={completed} max={total} aria-label={t("flow.export.progress")} />}
    <div className="flow-export-actions"><button type="button" data-export-start onClick={start} disabled={busy || !consent || !sourceValid || (format === "gif" && !valid)}>{t("flow.export.start")}</button>
      {busy && <button type="button" onClick={() => job.current?.abort()}>{t("flow.export.cancel")}</button>}
      <button type="button" onClick={props.onClose}>{t("flow.export.close")}</button></div>
    <small>GIF / Graphics Interchange Format: CompuServe Incorporated.</small>
  </Overlay>;
}
