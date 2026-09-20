/** Tests for the from-scratch project dialog: defaults, validation, and the 409 discard prompt. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  newProject: vi.fn(),
}));

// `errorMessage`/`isUnsavedChangesConflict` are re-implemented here rather
// than mocked away, because the dialog's whole 409 branch hangs off the
// second one — a `vi.fn()` returning `true` would test nothing. The real
// implementations are pinned against a real `fetch` response in
// `api.test.ts`; these two mirror them.
vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  isUnsavedChangesConflict: (error: unknown) =>
    error instanceof Error && (error as Error & { status?: number }).status === 409,
}));

import NewProjectDialog from "./NewProjectDialog";
import { settingsStorage } from "./settingsStore";
import { resetSettingsForTests } from "./settingsStore";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  window.localStorage.clear();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

function tree(): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    group_address_style: "ThreeLevel",
    installations: [],
  };
}

function httpError(status: number, message: string): Error {
  const error = new Error(message) as Error & { status: number };
  error.status = status;
  return error;
}

async function renderDialog() {
  const onCreated = vi.fn();
  const onClose = vi.fn();
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<NewProjectDialog onCreated={onCreated} onClose={onClose} />);
  });
  return { root, onCreated, onClose };
}

function field(label: string): HTMLInputElement {
  return host!.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
}

/** Goes through the native value setter so React's own change tracking
 * sees the write — the same idiom `BusComposeForm.test.tsx` documents. */
function setInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function setSelectValue(select: HTMLSelectElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value")!.set!;
  setter.call(select, value);
  select.dispatchEvent(new Event("change", { bubbles: true }));
}

function button(text: string): HTMLButtonElement {
  return Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === text)!;
}

/** What pressing Enter in any field does — the fast path this dialog is
 * built around, and the same event the Create button raises. */
async function submitForm() {
  const form = host!.querySelector("form")!;
  await act(async () => {
    form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  });
}

describe("NewProjectDialog", () => {
  it("opens inside the shared overlay shell with every field pre-filled and valid", async () => {
    const { root } = await renderDialog();

    const dialog = host!.querySelector('[role="dialog"]')!;
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(field("Project name").value).toBe("Untitled project");
    expect(field("Installation name").value).toBe("Installation 1");
    expect(field("Project language").value).toBe("en");
    expect(host!.querySelector("select")!.value).toBe("ThreeLevel");
    expect(button("Create project").disabled).toBe(false);
    expect(host!.querySelector(".field-error")).toBeNull();

    root.unmount();
  });

  it("submits the seeded defaults unchanged, never asking to discard anything", async () => {
    apiMock.newProject.mockResolvedValue(tree());
    const { root, onCreated } = await renderDialog();

    await submitForm();

    expect(apiMock.newProject).toHaveBeenCalledWith({
      name: "Untitled project",
      installationName: "Installation 1",
      language: "en",
      groupAddressStyle: "ThreeLevel",
      discardChanges: false,
    });
    expect(onCreated).toHaveBeenCalledWith(tree());

    root.unmount();
  });

  it("sends the edited fields and the chosen group address style, trimmed", async () => {
    apiMock.newProject.mockResolvedValue(tree());
    const { root } = await renderDialog();

    await act(async () => {
      setInputValue(field("Project name"), "  Workshop  ");
      setInputValue(field("Installation name"), "Cellar");
      setInputValue(field("Project language"), "de-DE");
      setSelectValue(host!.querySelector("select")!, "TwoLevel");
    });
    await submitForm();

    expect(apiMock.newProject).toHaveBeenCalledWith({
      name: "Workshop",
      installationName: "Cellar",
      language: "de-DE",
      groupAddressStyle: "TwoLevel",
      discardChanges: false,
    });

    root.unmount();
  });

  it("seeds the project language from the active UI language", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await renderDialog();

    expect(field("Projektsprache").value).toBe("de");
    expect(field("Projektname").value).toBe("Unbenanntes Projekt");

    root.unmount();
  });

  it("refuses to create a nameless project and says why", async () => {
    const { root } = await renderDialog();

    await act(async () => setInputValue(field("Project name"), "   "));

    expect(host!.textContent).toContain("A project needs a name.");
    expect(button("Create project").disabled).toBe(true);
    expect(field("Project name").getAttribute("aria-invalid")).toBe("true");
    await submitForm();
    expect(apiMock.newProject).not.toHaveBeenCalled();

    root.unmount();
  });

  it("refuses a malformed language tag", async () => {
    const { root } = await renderDialog();

    await act(async () => setInputValue(field("Project language"), "not a tag"));

    expect(host!.textContent).toContain("Not a well-formed language tag.");
    expect(button("Create project").disabled).toBe(true);
    await submitForm();
    expect(apiMock.newProject).not.toHaveBeenCalled();

    root.unmount();
  });

  it("turns the server's 409 into a question, and only an explicit answer discards", async () => {
    apiMock.newProject.mockRejectedValueOnce(
      httpError(409, "the open project has unsaved changes; save it first or resend with discardChanges: true"),
    );
    const { root, onCreated } = await renderDialog();

    await submitForm();

    // Nothing was created, and the user is told what is at stake in their
    // own language, with the server's own sentence kept underneath it.
    expect(onCreated).not.toHaveBeenCalled();
    const prompt = host!.querySelector('[role="alert"]')!;
    expect(prompt.textContent).toContain("The open project has unsaved changes");
    expect(prompt.textContent).toContain("no undo brings them back");
    expect(prompt.textContent).toContain("save it first or resend with discardChanges");
    expect(apiMock.newProject.mock.calls[0][0].discardChanges).toBe(false);

    // A second Enter while the question is up must not repeat the request
    // that already earned the 409.
    await submitForm();
    expect(apiMock.newProject).toHaveBeenCalledTimes(1);

    apiMock.newProject.mockResolvedValueOnce(tree());
    await act(async () => {
      button("Discard changes and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(apiMock.newProject).toHaveBeenCalledTimes(2);
    expect(apiMock.newProject.mock.calls[1][0].discardChanges).toBe(true);
    expect(onCreated).toHaveBeenCalledWith(tree());

    root.unmount();
  });

  it("backs out of the 409 without discarding when the user keeps editing", async () => {
    apiMock.newProject.mockRejectedValue(httpError(409, "the open project has unsaved changes"));
    const { root, onClose, onCreated } = await renderDialog();

    await submitForm();
    await act(async () => {
      button("Keep editing").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onCreated).not.toHaveBeenCalled();
    expect(apiMock.newProject).toHaveBeenCalledTimes(1);
    expect(
      apiMock.newProject.mock.calls.every((call) => call[0].discardChanges === false),
    ).toBe(true);

    root.unmount();
  });

  it("disables the discard button when the form behind the 409 prompt goes invalid", async () => {
    apiMock.newProject.mockRejectedValue(httpError(409, "the open project has unsaved changes"));
    const { root } = await renderDialog();

    await submitForm();
    expect(button("Discard changes and create").disabled).toBe(false);

    // The fields stay editable while the prompt is up. A blanked name makes
    // `submit()` return at its own guard, so a button that still looked
    // enabled would simply do nothing when pressed.
    await act(async () => setInputValue(field("Project name"), "   "));
    expect(button("Discard changes and create").disabled).toBe(true);

    await act(async () => {
      button("Discard changes and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(apiMock.newProject).toHaveBeenCalledTimes(1);

    root.unmount();
  });

  it("shows any other failure as an error and leaves the form usable", async () => {
    apiMock.newProject.mockRejectedValueOnce(httpError(400, "unknown groupAddressStyle"));
    const { root, onCreated } = await renderDialog();

    await submitForm();

    expect(host!.querySelector(".field-error")!.textContent).toBe("unknown groupAddressStyle");
    expect(host!.querySelector('[role="alert"]')).toBeNull();
    expect(onCreated).not.toHaveBeenCalled();
    expect(button("Create project").disabled).toBe(false);

    root.unmount();
  });
});
