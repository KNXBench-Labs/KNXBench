/** Hosts synthetic isolated parameter controls without storage or network writes. */
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "../../styles.css";
import { ParameterEditor } from "./ParameterEditor";
import type { ParameterEditorField, ParameterEditorSource } from "./plan";

const query = new URLSearchParams(location.search);
const locale = query.get("lang") === "de" ? "de" : "en";
const theme = query.get("theme") === "porcelain" ? "porcelain" : "graphite";
document.documentElement.lang = locale;
document.documentElement.dataset.theme = theme;
const sha = "a".repeat(64);
const context = { programId: "synthetic-A", sourceSha256: sha, moduleScoped: false };
const calls: Array<{ ref: string; raw: string }> = [];
Object.assign(window, { __parameterCalls: calls });
function Host() {
  const [revision, setRevision] = useState(0);
  const [values, setValues] = useState({ time: "0010", slider: "010" });
  const [refuse, setRefuse] = useState(false);
  const label = (en: string, de: string) => locale === "de" ? de : en;
  function numeric(ref: "time" | "slider"): ParameterEditorField {
    return { etsId: ref, kind: ref === "time" ? "Time" : "Number", value: values[ref], min: "0", max: ref === "time" ? "3600" : "100", editable: true, writeEtsId: ref, access: "ReadWrite" };
  }
  function metadata(ref: string, child: string, attributes: Record<string, string>): ParameterEditorSource {
    return { sourceSha256: sha, programId: context.programId, parameterRefId: ref, parameterTypeId: `PT-${ref}`, schema: 20, child, attributes, parentAttributes: { Id: `PT-${ref}` } };
  }
  async function apply(ref: string, raw: string) {
    calls.push({ ref, raw });
    if (refuse) throw new Error(label("Synthetic service refusal", "Ablehnung durch synthetischen Dienst"));
    return { ref, raw };
  }
  function applied(result: { ref: string; raw: string }) {
    setValues(previous => ({ ...previous, [result.ref]: result.raw })); setRevision(r => r + 1);
  }
  return <main style={{ maxWidth: 880, margin: "24px auto", padding: 16, background: "var(--knx-bg)", color: "var(--knx-foreground)", display: "grid", gap: 16 }}>
    <h1 style={{ margin: 0 }}>{label("Parameter controls", "Parametersteuerungen")}</h1>
    <p style={{ margin: 0 }}>{label("Synthetic component fixture — no API, project storage or hardware.", "Synthetische Komponentenansicht — keine API, Projektspeicherung oder Hardware.")}</p>
    <ParameterEditor field={numeric("time")} source={metadata("time", "TypeTime", { minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" })} context={context} selectionKey="synthetic/device1" revision={String(revision)} label={label("Delay", "Verzögerung")} locale={locale} onApply={apply} onApplied={applied} />
    <ParameterEditor field={numeric("slider")} source={metadata("slider", "TypeNumber", { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100" })} context={context} selectionKey="synthetic/device1" revision={String(revision)} label={label("Output level", "Ausgangswert")} locale={locale} onApply={apply} onApplied={applied} />
    {(["Color", "Picture"] as const).map(kind => <ParameterEditor key={kind} field={{ ...numeric("time"), etsId: kind, kind, min: null, max: null, value: kind === "Color" ? "<img src=\"https://untrusted.invalid/never\">" : "https://untrusted.invalid/diagram.png" }} source={metadata(kind, `Type${kind}`, kind === "Color" ? { Space: "RGB" } : { RefId: "unverified-picture" })} context={context} selectionKey="synthetic/device1" revision={String(revision)} label={kind === "Color" ? label("Colour", "Farbe") : label("Picture", "Bild")} locale={locale} onApply={apply} onApplied={applied} />)}
    <section style={{ display: "flex", gap: 12, flexWrap: "wrap" }} aria-label="Fixture controls">
      <button type="button" onClick={() => setRevision(r => r + 1)}>{label("Change fixture revision", "Testrevision ändern")}</button>
      <label><input type="checkbox" checked={refuse} onChange={e => setRefuse(e.target.checked)} />{label("Refuse next fixture edit", "Nächste Teständerung ablehnen")}</label>
    </section>
  </main>;
}
createRoot(document.getElementById("root")!).render(<Host />);
