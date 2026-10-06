/** Tests for the from-scratch project dialog: defaults, validation, and the 409 discard prompt. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import { importLanguagePack, removeLanguagePack, resetLanguagePacksForTests } from "./languagePack";
import { AVAILABLE_UI_LANGUAGES, resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";

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
  resetLanguagePacksForTests();
});

function tree(): ProjectTree {
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [],
  };
}

function httpError(status: number, message: string): Error {
  const error = new Error(message) as Error & { status: number };
  error.status = status;
  return error;
}

async function renderDialog(saveFirst: () => Promise<boolean> = () => Promise.resolve(false)) {
  const onCreated = vi.fn();
  const onClose = vi.fn();
  const onSaveFirst = vi.fn(saveFirst);
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<NewProjectDialog onCreated={onCreated} onClose={onClose} onSaveFirst={onSaveFirst} />);
  });
  return { root, onCreated, onClose, onSaveFirst };
}

function field(label: string): HTMLInputElement {
  return host!.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
}

function languageSelect(label = "Project language"): HTMLSelectElement {
  return host!.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`)!;
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
    expect(languageSelect().value).toBe("en");
    expect(languageSelect().getAttribute("aria-describedby")).toBe("new-project-language-hint");
    expect(host!.querySelector("#new-project-language-hint")?.textContent).toContain("Choose a listed language");
    expect([...languageSelect().options].map((option) => option.value)).toEqual([
      ...AVAILABLE_UI_LANGUAGES,
      "__custom__",
    ]);
    expect(host!.querySelector<HTMLSelectElement>('select[aria-label="Group address style"]')!.value).toBe("ThreeLevel");
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
      setSelectValue(languageSelect(), "__custom__");
      setSelectValue(host!.querySelector<HTMLSelectElement>('select[aria-label="Group address style"]')!, "TwoLevel");
    });
    await act(async () => setInputValue(field("Custom language tag"), "de-DE"));
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

    expect(languageSelect("Projektsprache").value).toBe("de");
    expect(field("Projektname").value).toBe("Unbenanntes Projekt");

    root.unmount();
  });

  it("offers installed language packs alongside the shared built-in language choices", async () => {
    expect(importLanguagePack({ formatVersion: 1, tag: "nl", name: "Nederlands", messages: {} }).ok).toBe(true);
    const { root } = await renderDialog();

    expect(languageSelect()).not.toBeNull();
    expect([...languageSelect().options].map((option) => [option.value, option.textContent])).toEqual([
      ["en", "English"],
      ["de", "Deutsch"],
      ["nl", "Nederlands"],
      ["__custom__", "Another language tag…"],
    ]);

    root.unmount();
  });

  it("uses an installed UI-language pack as the project's initial language", async () => {
    expect(importLanguagePack({ formatVersion: 1, tag: "nl", name: "Nederlands", messages: {} }).ok).toBe(true);
    saveUiLanguage(settingsStorage, "nl");
    resetUiLanguageForTests();
    const { root } = await renderDialog();

    expect(languageSelect().value).toBe("nl");
    expect(field("Custom language tag")).toBeNull();
    root.unmount();
  });

  it("preserves the selected project tag when its UI-language pack is removed", async () => {
    expect(importLanguagePack({ formatVersion: 1, tag: "nl", name: "Nederlands", messages: {} }).ok).toBe(true);
    apiMock.newProject.mockResolvedValue(tree());
    const { root } = await renderDialog();

    await act(async () => setSelectValue(languageSelect(), "nl"));
    await act(async () => removeLanguagePack("nl"));
    expect(languageSelect().value).toBe("__custom__");
    expect(field("Custom language tag").value).toBe("nl");
    await submitForm();
    expect(apiMock.newProject.mock.calls[0][0].language).toBe("nl");

    root.unmount();
  });

  it("does not offer a shadowed installed pack twice for a built-in language", async () => {
    expect(importLanguagePack({ formatVersion: 1, tag: "en", name: "Shadow English", messages: {} }).ok).toBe(true);
    const { root } = await renderDialog();
    expect([...languageSelect().options].map((option) => option.value)).toEqual([
      ...AVAILABLE_UI_LANGUAGES,
      "__custom__",
    ]);
    root.unmount();
  });

  it("preserves an active custom UI-language tag as the project default", async () => {
    saveUiLanguage(settingsStorage, "art-x-sindarin");
    resetUiLanguageForTests();
    apiMock.newProject.mockResolvedValue(tree());
    const { root } = await renderDialog();

    expect(languageSelect().value).toBe("__custom__");
    expect(field("Custom language tag").value).toBe("art-x-sindarin");
    await submitForm();
    expect(apiMock.newProject.mock.calls[0][0].language).toBe("art-x-sindarin");

    root.unmount();
  });

  it("says that Save or Save As chooses the .knxdb filename, not either name", async () => {
    const { root } = await renderDialog();
    expect(host!.querySelector(".new-project-intro")?.nextElementSibling?.classList.contains("new-project-filename-hint")).toBe(true);
    expect(field("Project name").getAttribute("aria-describedby")).toBe("new-project-filename-hint");
    expect(field("Installation name").getAttribute("aria-describedby")).toBe("new-project-filename-hint");
    expect(host!.querySelector(".new-project-filename-hint")?.textContent).toContain(
      "Save or Save As chooses the .knxdb filename. Project and installation names are not file paths.",
    );
    root.unmount();
  });

  it("explains the filename decision in German too", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root } = await renderDialog();
    expect(host!.querySelector(".new-project-filename-hint")?.textContent).toContain(
      "Speichern oder Speichern unter legt den .knxdb-Dateinamen fest. Projekt- und Anlagenname sind keine Dateipfade.",
    );
    root.unmount();
  });

  // Since 2026-10-02 the Project node's style select restyles a project as one
  // undoable command (`setGroupAddressStyle`); the hint must not claim the
  // choice is final (AR16 hand-over, 2026-10-06).
  it("says the group-address style can be changed later on the Project node", async () => {
    const { root } = await renderDialog();
    expect(host!.textContent).toContain("You can change the group-address style later on the Project node.");
    expect(host!.textContent).not.toContain("cannot currently be changed");
    root.unmount();

    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    const { root: germanRoot } = await renderDialog();
    expect(host!.textContent).toContain("Den Gruppenadressstil kannst du später am Projektknoten ändern.");
    expect(host!.textContent).not.toContain("nicht geändert werden");
    germanRoot.unmount();
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

    await act(async () => setSelectValue(languageSelect(), "__custom__"));
    await act(async () => setInputValue(field("Custom language tag"), "not a tag"));

    expect(host!.textContent).toContain("Not a well-formed language tag.");
    expect(button("Create project").disabled).toBe(true);
    expect(field("Custom language tag").getAttribute("aria-invalid")).toBe("true");
    expect(field("Custom language tag").getAttribute("aria-describedby")).toBe("new-project-language-hint new-project-language-error");
    expect(host!.querySelector("#new-project-language-error")?.getAttribute("role")).toBe("alert");
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

  it("saves first and then creates, never sending discardChanges (ISSUE-04 Save-and-create)", async () => {
    apiMock.newProject.mockRejectedValueOnce(httpError(409, "the open project has unsaved changes"));
    const { root, onCreated, onSaveFirst } = await renderDialog(() => Promise.resolve(true));

    await submitForm();
    expect(onSaveFirst).not.toHaveBeenCalled();

    apiMock.newProject.mockResolvedValueOnce(tree());
    await act(async () => {
      button("Save and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onSaveFirst).toHaveBeenCalledTimes(1);
    expect(apiMock.newProject).toHaveBeenCalledTimes(2);
    expect(apiMock.newProject.mock.calls.every((call) => call[0].discardChanges === false)).toBe(true);
    expect(onCreated).toHaveBeenCalledWith(tree());

    root.unmount();
  });

  it("keeps the prompt up and creates nothing when the save fails or is cancelled", async () => {
    apiMock.newProject.mockRejectedValueOnce(httpError(409, "the open project has unsaved changes"));
    const { root, onCreated, onSaveFirst } = await renderDialog(() => Promise.resolve(false));

    await submitForm();
    await act(async () => {
      button("Save and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onSaveFirst).toHaveBeenCalledTimes(1);
    expect(apiMock.newProject).toHaveBeenCalledTimes(1);
    expect(onCreated).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();
    expect(button("Save and create").disabled).toBe(false);

    root.unmount();
  });

  it("stays in the prompt when the project is still dirty after saving", async () => {
    // The save reported success, but the server still refuses: e.g. an edit
    // landed between the save and the create. Nothing may be discarded.
    apiMock.newProject.mockRejectedValue(httpError(409, "the open project has unsaved changes"));
    const { root, onCreated } = await renderDialog(() => Promise.resolve(true));

    await submitForm();
    await act(async () => {
      button("Save and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(apiMock.newProject).toHaveBeenCalledTimes(2);
    expect(apiMock.newProject.mock.calls.every((call) => call[0].discardChanges === false)).toBe(true);
    expect(onCreated).not.toHaveBeenCalled();
    expect(host!.querySelector('[role="alert"]')).not.toBeNull();

    root.unmount();
  });

  it("creates nothing when the dialog is dismissed while Save and create is still saving", async () => {
    apiMock.newProject.mockRejectedValueOnce(httpError(409, "the open project has unsaved changes"));
    let finishSave!: (saved: boolean) => void;
    const { root, onCreated, onClose } = await renderDialog(
      () => new Promise<boolean>((resolve) => { finishSave = resolve; }),
    );

    await submitForm();
    await act(async () => {
      button("Save and create").dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await act(async () => {
      host!
        .querySelector('[role="dialog"]')!
        .dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    await act(async () => {
      finishSave(true);
    });
    expect(apiMock.newProject).toHaveBeenCalledTimes(1);
    expect(onCreated).not.toHaveBeenCalled();

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
