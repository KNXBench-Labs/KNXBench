/** AR08: the project-password dialog asks, explains, and keeps the secret only in the field. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import ProjectPasswordDialog from "./ProjectPasswordDialog";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  resetSettingsForTests();
  resetUiLanguageForTests();
});

async function render(reason: "required" | "wrong", onSubmit = vi.fn(), onCancel = vi.fn()) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(
    <ProjectPasswordDialog fileName="villa.knxproj" reason={reason} onSubmit={onSubmit} onCancel={onCancel} />,
  ));
  return { onSubmit, onCancel };
}

function type(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

const field = () => document.querySelector<HTMLInputElement>('input[type="password"]')!;
const button = (label: string) => Array.from(document.querySelectorAll("button")).find((b) => b.textContent === label)!;

describe("ProjectPasswordDialog", () => {
  it("asks for the password of the named project in a masked, non-remembered field", async () => {
    await render("required");
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain("villa.knxproj");
    expect(document.body.textContent).toContain("password-protected");
    expect(document.body.textContent).toContain("not stored");
    expect(field().getAttribute("autocomplete")).toBe("off");
    expect(document.activeElement).toBe(field());
    expect(button("Import").disabled).toBe(true);
  });

  it("says when a password was not accepted", async () => {
    await render("wrong");
    expect(document.body.textContent).toContain("was not accepted");
  });

  it("hands the entered password over once and empties the field", async () => {
    const { onSubmit } = await render("required");
    await act(async () => type(field(), "s3cret"));
    await act(async () => button("Import").click());
    expect(onSubmit).toHaveBeenCalledExactlyOnceWith("s3cret");
    expect(field().value).toBe("");
  });

  it("submits with Enter and cancels without handing anything over", async () => {
    const { onSubmit, onCancel } = await render("required");
    await act(async () => type(field(), "abc"));
    await act(async () => field().form!.requestSubmit());
    expect(onSubmit).toHaveBeenCalledExactlyOnceWith("abc");
    await act(async () => button("Cancel").click());
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("speaks German", async () => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
    await render("wrong");
    expect(document.body.textContent).toContain("nicht akzeptiert");
    expect(button("Importieren")).toBeTruthy();
  });
});
