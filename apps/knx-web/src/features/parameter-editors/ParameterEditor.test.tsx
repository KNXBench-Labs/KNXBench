/** Exercises isolated parameter controls with synthetic source evidence and explicit writes. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ParameterEditor, type ParameterEditorProps } from "./ParameterEditor";
import ipVectors from "../../../../../crates/knx-productdb/tests/fixtures/ip-address-literal.json";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const sha = "a".repeat(64);
const base: ParameterEditorProps<{ tree: string }> = {
  field: { etsId: "PR", kind: "Time", value: "0010", min: "0", max: "3600", editable: true, writeEtsId: "PR", access: "ReadWrite" },
  source: { sourceSha256: sha, programId: "A", parameterRefId: "PR", parameterTypeId: "PT", schema: 20, child: "TypeTime", attributes: { minInclusive: "0", maxInclusive: "3600", Unit: "Seconds" }, parentAttributes: { Id: "PT" } },
  context: { programId: "A", sourceSha256: sha, moduleScoped: false },
  selectionKey: "project1/device1", revision: "server1/snapshot1", label: "Delay", locale: "en",
  onApply: async () => ({ tree: "authoritative" }), onApplied: () => {},
};
const roots = new Set<Root>();
afterEach(async () => { await act(async () => { for (const root of roots) root.unmount(); }); roots.clear(); document.body.replaceChildren(); });
async function mount(extra: Partial<typeof base> = {}) {
  const props = { ...base, onApply: vi.fn(base.onApply), onApplied: vi.fn(base.onApplied), ...extra };
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host); roots.add(root);
  await act(async () => root.render(<ParameterEditor {...props} />)); return { props, root, host };
}
async function edit(host: Element, value: string, type = "text") {
  const input = host.querySelector<HTMLInputElement>(`input[type="${type}"]`); expect(input).not.toBeNull();
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value); input!.dispatchEvent(new Event("input", { bubbles: true })); });
}
async function click(host: Element, label: string) {
  const button = Array.from(host.querySelectorAll("button")).find(b => b.textContent === label); expect(button).toBeDefined(); await act(async () => button!.click());
}
async function pasteSelection(input: HTMLInputElement, raw: string, start = 0, end = input.value.length) {
  await act(async () => {
    input.focus(); input.setSelectionRange(start, end);
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", { value: { getData: () => raw } });
    input.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
  });
}
const ip = {
  field: { ...base.field, kind: "IPAddress", min: null, max: null, value: "203.0.113.19", writeEtsId: "WRITE-IP" },
  source: { ...base.source!, child: "TypeIPAddress", attributes: { AddressType: "HostAddress" } },
  label: "Endpoint",
};
describe("OKX06 IP literal component", () => {
  it.each(["\n", "\r", "\r\n"])("R1 keeps ambiguous partial paste in historical raw %j inert until full replacement", async newline => {
    const original = `203.0.113.${newline}19`;
    const { host, props } = await mount({ ...ip, field: { ...ip.field, value: original } });
    const input = host.querySelector<HTMLInputElement>('input[type="text"]')!;
    expect(input.value).toBe("203.0.113.19");
    await pasteSelection(input, "20", input.value.lastIndexOf(".") + 1);
    await click(host, "Apply value");
    expect(props.onApply).not.toHaveBeenCalled();
    expect(input.value).toBe("203.0.113.19");
    expect(host.querySelector('[aria-label="Unchanged original value"]')?.textContent).toBe(original);
    expect(host.textContent).toContain("Select the entire displayed value and paste its complete replacement");
    await pasteSelection(input, "203.0.113.20");
    await click(host, "Apply value");
    expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", "203.0.113.20");
  });
  it.each(["\n", "\r", "\r\n"])("R1 select-all replaces all raw content after a malformed %j clipboard draft", async newline => {
    const { host, props } = await mount(ip);
    const input = host.querySelector<HTMLInputElement>('input[type="text"]')!;
    await pasteSelection(input, `198.51.100.254${newline}`);
    expect(host.querySelector('[aria-label="Unchanged invalid draft"]')?.textContent).toBe(`198.51.100.254${newline}`);
    await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
    await pasteSelection(input, "203.0.113.20");
    expect(input.value).toBe("203.0.113.20");
    await click(host, "Apply value");
    expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", "203.0.113.20");
  });
  it("R1 control preserves ordinary single-line partial replacement", async () => {
    const { host, props } = await mount(ip);
    const input = host.querySelector<HTMLInputElement>('input[type="text"]')!;
    await pasteSelection(input, "198", 0, 3);
    expect(input.value).toBe("198.0.113.19");
    await click(host, "Apply value");
    expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", "198.0.113.19");
  });
  it.each(ipVectors.cases)("pastes and preflights exact raw $raw => $accepted without normalization", async ({ raw, accepted }) => {
    const { host, props } = await mount({ ...ip, field: { ...ip.field, value: null } });
    const input = host.querySelector<HTMLInputElement>("input")!;
    await act(async () => {
      input.focus(); input.setSelectionRange(0, input.value.length);
      const event = new Event("paste", { bubbles: true, cancelable: true });
      Object.defineProperty(event, "clipboardData", { value: { getData: () => raw } });
      input.dispatchEvent(event);
    });
    await click(host, "Apply value");
    if (accepted) expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", raw);
    else { expect(props.onApply).not.toHaveBeenCalled(); if (raw !== "") expect(host.querySelector('[role="alert"]')).not.toBeNull(); }
    if (raw.includes("\n")) expect(host.textContent).toContain(raw);
  });
  it("shows text input and format help; opening, no-op and blur never write", async () => {
    const { host, props } = await mount(ip);
    const input = host.querySelector<HTMLInputElement>('input[type="text"]')!;
    expect(input).not.toBeNull(); expect(input.inputMode).toBe("text"); expect(input.value).toBe(ip.field.value);
    expect(host.textContent).toContain("eight groups of four hexadecimal digits");
    expect(host.querySelector('input[type="number"],input[type="url"],img,a')).toBeNull();
    await act(async () => { input.focus(); input.blur(); }); await click(host, "Apply value");
    expect(props.onApply).not.toHaveBeenCalled();
  });
  it("applies exact mixed-case IPv6 once and publishes the actual service result", async () => {
    const { host, props, root } = await mount(ip);
    const raw = "2001:0DB8:0012:0345:0067:0089:00Ab:CDEF";
    await edit(host, raw); expect(props.onApply).not.toHaveBeenCalled();
    await click(host, "Apply value"); await click(host, "Apply value");
    expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", raw);
    expect(props.onApplied).toHaveBeenCalledExactlyOnceWith({ tree: "authoritative" });
    await act(async () => root.render(<ParameterEditor {...props} revision="next" field={{ ...ip.field, value: raw }} />));
    expect(host.querySelector<HTMLInputElement>("input")!.value).toBe(raw);
    await act(async () => root.render(<ParameterEditor {...props} revision="undo" />));
    expect(host.querySelector<HTMLInputElement>("input")!.value).toBe(ip.field.value);
  });
  it("shows malformed drafts inline and keeps exact service refusal until explicit Reset", async () => {
    const onApply = vi.fn().mockRejectedValue(new Error("Exact service refusal"));
    const { host, props } = await mount({ ...ip, locale: "de", onApply });
    await edit(host, "2001:db8::1"); await click(host, "Wert übernehmen");
    expect(host.querySelector('[role="alert"]')?.textContent).toContain("IPv4"); expect(onApply).not.toHaveBeenCalled();
    await edit(host, "198.51.100.254"); await click(host, "Wert übernehmen");
    expect(host.textContent).toContain("Exact service refusal"); expect(props.onApplied).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>("input")!.value).toBe("198.51.100.254");
    await click(host, "Entwurf verwerfen und neu laden"); expect(host.querySelector<HTMLInputElement>("input")!.value).toBe(ip.field.value);
  });
  it.each([null, "", "historical invalid", "203.0.113.19\n"])("never repairs historical %j on load", async value => {
    const { host, props } = await mount({ ...ip, field: { ...ip.field, value } });
    await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
    if (value !== null && value !== "") expect(host.textContent).toContain(value);
    if (value === null) expect(host.textContent).toContain("No value is present.");
    if (value === "") expect(host.textContent).toContain("The stored value is explicitly empty.");
    await edit(host, "0.0.0.0"); await click(host, "Apply value");
    expect(props.onApply).toHaveBeenCalledExactlyOnceWith("WRITE-IP", "0.0.0.0");
  });
  it.each([{ editable: false }, { writeEtsId: null }, { access: "Read" }, { access: "None" }])("retains authority %j", async restriction => {
    const { host, props } = await mount({ ...ip, field: { ...ip.field, ...restriction } });
    expect(host.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    await edit(host, "255.255.255.255"); await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
  });
  it("blocks stale drafts until reset and suppresses double-click and late settlement", async () => {
    let settle!: (value: { tree: string }) => void;
    const onApply = vi.fn().mockReturnValue(new Promise(done => { settle = done; }));
    const { host, props, root } = await mount({ ...ip, onApply }); await edit(host, "198.51.100.254");
    const apply = [...host.querySelectorAll("button")].find(b => b.textContent === "Apply value")!;
    await act(async () => { apply.click(); apply.click(); }); expect(onApply).toHaveBeenCalledTimes(1);
    await act(async () => root.render(<ParameterEditor {...props} revision="new" field={{ ...ip.field, value: "0.0.0.0" }} />));
    await act(async () => settle({ tree: "old" })); expect(props.onApplied).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>("input")!.value).toBe("198.51.100.254");
    await click(host, "Apply value"); expect(onApply).toHaveBeenCalledTimes(1);
    await click(host, "Discard draft and reload"); expect(host.querySelector<HTMLInputElement>("input")!.value).toBe("0.0.0.0");
    await act(async () => root.render(<ParameterEditor {...props} selectionKey="other-device" />));
    expect(host.querySelector<HTMLInputElement>("input")!.value).toBe(ip.field.value);
  });
});

describe("isolated parameter editor", () => {
  it("suppresses a pending response after a revision change without allowing a duplicate write", async () => {
    let settle!: (value: { tree: string }) => void;
    const onApply = vi.fn().mockReturnValue(new Promise(done => { settle = done; }));
    const { host, props, root } = await mount({ onApply }); await edit(host, "12");
    const button = Array.from(host.querySelectorAll("button")).find(b => b.textContent === "Apply value")!;
    await act(async () => { button.click(); button.click(); }); expect(onApply).toHaveBeenCalledTimes(1);
    await act(async () => root.render(<ParameterEditor {...props} revision="new-revision" />));
    await act(async () => settle({ tree: "old" })); expect(props.onApplied).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12");
    expect(host.textContent).toContain("draft is retained");
  });
  it("does not publish an outcome after unmount", async () => {
    let settle!: (value: { tree: string }) => void;
    const onApply = vi.fn().mockReturnValue(new Promise(done => { settle = done; }));
    const { host, props, root } = await mount({ onApply }); await edit(host, "12"); await click(host, "Apply value");
    await act(async () => root.unmount()); roots.delete(root);
    await act(async () => settle({ tree: "old" })); expect(props.onApplied).not.toHaveBeenCalled();
  });
  it("does not submit twice while awaiting the host's authoritative refresh", async () => {
    const { host, props } = await mount(); await edit(host, "12"); await click(host, "Apply value");
    const apply = Array.from(host.querySelectorAll("button")).find(b => b.textContent === "Apply value")!;
    expect(apply.disabled).toBe(true); await click(host, "Apply value"); expect(props.onApply).toHaveBeenCalledTimes(1);
  });
  it("preserves the exact time-count lexeme and sends only an explicit changed draft", async () => {
    const { host, props } = await mount();
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("0010");
    expect(host.textContent).toContain("Seconds"); expect(host.textContent).toContain("no time conversion");
    expect(props.onApply).not.toHaveBeenCalled();
    await edit(host, "+0012"); expect(props.onApply).not.toHaveBeenCalled();
    await click(host, "Apply value"); expect(props.onApply).toHaveBeenCalledExactlyOnceWith("PR", "+0012"); expect(props.onApplied).toHaveBeenCalledExactlyOnceWith({ tree: "authoritative" });
  });
  it("leaves no-op, invalid and manufacturer-restricted values unwritten", async () => {
    const { host, props, root } = await mount(); await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
    await edit(host, "1.5"); await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled(); expect(host.textContent).toContain("integer");
    await act(async () => root.render(<ParameterEditor {...props} field={{ ...base.field, access: "Read" }} />));
    await edit(host, "12"); await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled(); expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.disabled).toBe(true);
  });
  it("retains a rejected draft and exposes the exact service refusal", async () => {
    const onApply = vi.fn().mockRejectedValue(new Error("Native storage refused")); const { host, props } = await mount({ onApply });
    await edit(host, "12"); await click(host, "Apply value"); expect(host.textContent).toContain("Native storage refused");
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12"); expect(props.onApplied).not.toHaveBeenCalled();
  });
  it("keeps a changed-revision draft visible but blocked until deliberate reload", async () => {
    const { host, props, root } = await mount(); await edit(host, "12");
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" field={{ ...base.field, value: "20" }} />));
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12"); await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
    const discard = [...host.querySelectorAll("button")].find(button => button.textContent === "Discard draft and reload");
    expect(discard?.classList.contains("danger-action")).toBe(true);
    await click(host, "Discard draft and reload"); expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("20");
    expect([...host.querySelectorAll("button")].find(button => button.textContent === "Discard draft and reload")?.classList.contains("danger-action")).toBe(false);
  });
  it("moves a draft onto a new revision when the field itself is unchanged (Save bumps the revision)", async () => {
    const { host, props, root } = await mount(); await edit(host, "12");
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" />));
    expect(host.textContent).not.toContain("draft is retained");
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("12");
    await click(host, "Apply value"); expect(props.onApply).toHaveBeenCalledWith("PR", "12");
  });
  it("still blocks a draft when the field changed together with the revision", async () => {
    const { host, props, root } = await mount(); await edit(host, "12");
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" field={{ ...base.field, access: "ReadWrite", editable: true, value: "0011" }} />));
    expect(host.textContent).toContain("draft is retained");
    await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
  });
  it("blocks a draft when only the declared range changed with the revision", async () => {
    const { host, props, root } = await mount(); await edit(host, "12");
    const source = { ...base.source!, attributes: { ...base.source!.attributes, minInclusive: "1" } };
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" source={source} field={{ ...base.field, min: "1" }} />));
    expect(host.textContent).toContain("draft is retained");
    await click(host, "Apply value"); expect(props.onApply).not.toHaveBeenCalled();
  });
  it("settles when the authoritative refresh carries exactly the applied value", async () => {
    const { host, props, root } = await mount(); await edit(host, "12"); await click(host, "Apply value");
    expect(props.onApplied).toHaveBeenCalledTimes(1);
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" field={{ ...base.field, value: "12" }} />));
    expect(host.textContent).not.toContain("draft is retained");
    expect(host.textContent).toContain("Applied. The field shows the project's value.");
    const input = host.querySelector<HTMLInputElement>('input[type="text"]')!;
    expect(input.value).toBe("12"); expect(input.disabled).toBe(false);
    await edit(host, "13"); expect(host.textContent).not.toContain("Applied. The field shows");
    await click(host, "Apply value"); expect(props.onApply).toHaveBeenLastCalledWith("PR", "13");
  });
  it("never rebases onto a revision that arrived while its own write was in flight", async () => {
    let settle!: (value: { tree: string }) => void;
    const onApply = vi.fn().mockReturnValue(new Promise(done => { settle = done; }));
    const { host, props, root } = await mount({ onApply }); await edit(host, "12"); await click(host, "Apply value");
    await act(async () => root.render(<ParameterEditor {...props} onApply={onApply} revision="server1/snapshot2" />));
    await act(async () => settle({ tree: "old" }));
    await edit(host, "14");
    expect(host.textContent).toContain("draft is retained");
    await click(host, "Apply value"); expect(onApply).toHaveBeenCalledTimes(1);
  });
  it("follows a changed authoritative value when there is no draft", async () => {
    const { host, props, root } = await mount();
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" field={{ ...base.field, value: "20" }} />));
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("20");
    expect(host.textContent).not.toContain("draft is retained");
  });
  it("follows Undo after a settled Apply instead of holding the applied value", async () => {
    const { host, props, root } = await mount(); await edit(host, "12"); await click(host, "Apply value");
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot2" field={{ ...base.field, value: "12" }} />));
    await act(async () => root.render(<ParameterEditor {...props} revision="server1/snapshot3" field={{ ...base.field, value: "0010" }} />));
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("0010");
    expect(host.textContent).not.toContain("draft is retained");
    expect(host.textContent).not.toContain("Applied. The field shows");
  });
  it("ignores a late result after selection change", async () => {
    let settle!: (value: { tree: string }) => void; const onApply = vi.fn().mockReturnValue(new Promise(done => { settle = done; }));
    const { host, props, root } = await mount({ onApply }); await edit(host, "12"); await click(host, "Apply value");
    await act(async () => root.render(<ParameterEditor {...props} selectionKey="project1/device2" label="Other" field={{ ...base.field, value: "30" }} />));
    await act(async () => settle({ tree: "old" })); expect(props.onApplied).not.toHaveBeenCalled(); expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("30");
  });
  it("renders a slider draft without an automatic write", async () => {
    const field = { ...base.field, kind: "Number", min: "0", max: "100" };
    const source = { ...base.source!, child: "TypeNumber", attributes: { Type: "unsignedInt", UIHint: "Slider", minInclusive: "0", maxInclusive: "100" } };
    const { host, props } = await mount({ field, source }); await edit(host, "40", "range"); expect(props.onApply).not.toHaveBeenCalled();
    expect(host.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe("40"); await click(host, "Apply value"); expect(props.onApply).toHaveBeenCalledExactlyOnceWith("PR", "40");
  });
  it("shows explicit German colour fallback without resources or execution", async () => {
    const raw = '<img src="https://untrusted.invalid/image">'; const field = { ...base.field, kind: "Color", value: raw, min: null, max: null };
    const source = { ...base.source!, child: "TypeColor", attributes: { Space: "RGB" } };
    const { host, props } = await mount({ field, source, locale: "de" });
    expect(host.textContent).toContain("Wertformat"); expect(host.textContent).toContain(raw); expect(host.querySelector('img,iframe,script,input[type="color"],input[type="file"]')).toBeNull();
    expect(props.onApply).not.toHaveBeenCalled();
  });
  it("shows a packed time value unchanged and offers no count input (census G6)", async () => {
    const source = { ...base.source!, attributes: { ...base.source!.attributes, Unit: "PackedSecondsAndMilliseconds" } };
    const { host, props } = await mount({ source });
    expect(host.querySelector("input")).toBeNull();
    expect(host.textContent).toContain("This value format is not verified. The original value is shown without conversion.");
    expect(host.textContent).toContain("0010");
    expect(host.textContent).not.toContain("Decimal count");
    expect(props.onApply).not.toHaveBeenCalled();
  });
});
