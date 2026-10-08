/** Tests for the new-project wizard: defaults, steps, structure, validation and the 409 prompt. */
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
  const onAddDevices = vi.fn();
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <NewProjectDialog onCreated={onCreated} onClose={onClose} onSaveFirst={onSaveFirst} onAddDevices={onAddDevices} />,
    );
  });
  return { root, onCreated, onClose, onSaveFirst, onAddDevices };
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

/** Q2 of the wizard interview: area 1 with line 1.1 and nothing else. */
const DEFAULT_SEED = {
  areas: [{ name: "Area 1", address: 1, lines: [{ name: "Line 1.1", address: 1, mediumRef: "MT-0" }] }],
  buildings: [],
  groupRanges: [],
};

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
      "bar", "tlh",
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
      seed: DEFAULT_SEED,
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
      seed: DEFAULT_SEED,
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
      ["bar", "Boarisch"],
      ["tlh", "Klingonisch/Klingon"],
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
      "bar", "tlh",
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

  describe("wizard steps (ADR-0093)", () => {
    function click(target: HTMLElement) {
      return act(async () => {
        target.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
    }

    function stepButton(label: string): HTMLButtonElement {
      const steps = host!.querySelector('ol[aria-label="Wizard steps"]')!;
      return Array.from(steps.querySelectorAll("button")).find((b) => b.textContent?.includes(label))!;
    }

    function labelled(label: string): HTMLInputElement[] {
      return Array.from(host!.querySelectorAll<HTMLInputElement>(`input[aria-label="${label}"]`));
    }

    function treeWithLine(lineId: number): ProjectTree {
      return {
        ...tree(),
        installations: [{
          id: 0, name: "Installation 1", buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
          topology: [{ id: 1, name: "Area 1", address: 1, lines: [{ id: lineId, name: "Line 1.1", address: 1, devices: [] }] }],
        }],
      };
    }

    it("walks the steps with Next and Back, starting from the pre-filled topology", async () => {
      const { root } = await renderDialog();
      expect(host!.querySelector("h3")!.textContent).toBe("Step 1 of 5: Project");
      expect(stepButton("Project").getAttribute("aria-current")).toBe("step");

      await click(button("Next"));
      expect(host!.querySelector("h3")!.textContent).toBe("Step 2 of 5: Topology");
      expect(labelled("Area number").map((i) => i.value)).toEqual(["1"]);
      expect(labelled("Area name").map((i) => i.value)).toEqual(["Area 1"]);
      expect(labelled("Line number").map((i) => i.value)).toEqual(["1"]);
      expect(labelled("Line name").map((i) => i.value)).toEqual(["Line 1.1"]);
      expect(labelled("Medium reference").map((i) => i.value)).toEqual(["MT-0"]);

      await click(button("Back"));
      expect(field("Project name").value).toBe("Untitled project");
      expect(button("Back").disabled).toBe(true);
      root.unmount();
    });

    it("builds a building with quick-filled floors and a floor-derived group preset, and sends it as the seed", async () => {
      apiMock.newProject.mockResolvedValue(tree());
      const { root } = await renderDialog();

      await click(stepButton("Building"));
      await click(button("Add building"));
      expect(labelled("Building name").map((i) => i.value)).toEqual(["Building 1"]);
      await act(async () => setInputValue(host!.querySelector<HTMLInputElement>(".project-wizard-quick input")!, "2"));
      await click(button("Add floors"));
      expect(labelled("Floor name").map((i) => i.value)).toEqual(["Floor 1", "Floor 2"]);
      await act(async () => setInputValue(labelled("Floor name")[0], "Ground"));
      await click(Array.from(host!.querySelectorAll("button")).filter((b) => b.textContent === "Add room")[0]);
      expect(labelled("Room name").map((i) => i.value)).toEqual(["Room 1"]);

      await click(stepButton("Group structure"));
      // The first preset is "function, then floor": functions become main
      // groups from 1, the building step's floors become middle groups from 0.
      await click(button("Apply preset (replaces the list)"));
      expect(labelled("Main group name").map((i) => i.value)).toEqual([
        "Lighting", "Shading", "Heating", "Ventilation", "Central functions",
      ]);
      expect(labelled("Main group number").map((i) => i.value)).toEqual(["1", "2", "3", "4", "5"]);
      expect(labelled("Middle group name").slice(0, 2).map((i) => i.value)).toEqual(["Ground", "Floor 2"]);

      await click(button("Create project"));
      const seed = apiMock.newProject.mock.calls[0][0].seed;
      expect(seed.areas).toEqual(DEFAULT_SEED.areas);
      expect(seed.buildings).toEqual([{
        name: "Building 1", kind: "Building", children: [
          { name: "Ground", kind: "Floor", children: [{ name: "Room 1", kind: "Room", children: [] }] },
          { name: "Floor 2", kind: "Floor", children: [] },
        ],
      }]);
      expect(seed.groupRanges).toHaveLength(5);
      expect(seed.groupRanges[0]).toEqual({
        name: "Lighting", main: 1, middles: [{ name: "Ground", middle: 0 }, { name: "Floor 2", middle: 1 }],
      });
      root.unmount();
    });

    it("refuses a floor-based preset without floors and says why, changing nothing", async () => {
      const { root } = await renderDialog();
      await click(stepButton("Group structure"));
      await act(async () => setSelectValue(host!.querySelector<HTMLSelectElement>(".project-wizard-preset select")!, "floor-function"));
      await click(button("Apply preset (replaces the list)"));
      expect(host!.textContent).toContain("This preset uses the floors from the Building step.");
      expect(labelled("Main group name")).toHaveLength(0);
      root.unmount();
    });

    it("blocks Create on a structure problem, names it, and links Review back to its step", async () => {
      const { root } = await renderDialog();
      await click(stepButton("Topology"));
      await click(button("Add line"));
      expect(labelled("Line number").map((i) => i.value)).toEqual(["1", "2"]);
      await act(async () => setInputValue(labelled("Line number")[1], "1"));

      expect(host!.textContent).toContain("Number 1 is already used here.");
      expect(labelled("Line number")[1].getAttribute("aria-invalid")).toBe("true");
      expect(button("Create project").disabled).toBe(true);
      expect(stepButton("Topology").textContent).toContain("1 to fix");
      expect(host!.textContent).toContain("1 entry needs fixing before the project can be created.");

      await click(stepButton("Review"));
      await click(button("Go to Topology"));
      expect(host!.querySelector("h3")!.textContent).toBe("Step 2 of 5: Topology");
      await click(button("Create project"));
      expect(apiMock.newProject).not.toHaveBeenCalled();
      root.unmount();
    });

    it("skips the group step for free style and limits two-level style to main groups", async () => {
      const { root } = await renderDialog();
      const style = () => host!.querySelector<HTMLSelectElement>('select[aria-label="Group address style"]')!;
      await act(async () => setSelectValue(style(), "Free"));
      expect(host!.querySelector("h3")!.textContent).toBe("Step 1 of 4: Project");
      expect(stepButton("Group structure")).toBeUndefined();

      await act(async () => setSelectValue(style(), "TwoLevel"));
      await click(stepButton("Group structure"));
      expect(host!.textContent).toContain("Two-level style: main groups only.");
      await click(button("Add main group"));
      expect(labelled("Main group number").map((i) => i.value)).toEqual(["1"]);
      expect(button("Add middle group")).toBeUndefined();
      root.unmount();
    });

    it("never creates on Enter inside a structure editor", async () => {
      apiMock.newProject.mockResolvedValue(tree());
      const { root } = await renderDialog();
      await click(stepButton("Topology"));
      const input = labelled("Area name")[0];
      const enter = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
      await act(async () => {
        input.dispatchEvent(enter);
      });
      expect(enter.defaultPrevented).toBe(true);
      await submitForm();
      expect(apiMock.newProject).not.toHaveBeenCalled();
      root.unmount();
    });

    it("asks before discarding entered data, and Escape on the question keeps editing", async () => {
      const { root, onClose } = await renderDialog();
      const escape = () => act(async () => {
        host!.querySelector('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      });
      await escape();
      expect(onClose).toHaveBeenCalledTimes(1);
      onClose.mockClear();

      await act(async () => setInputValue(field("Project name"), "Villa"));
      await escape();
      expect(onClose).not.toHaveBeenCalled();
      expect(host!.textContent).toContain("Discard your entries?");
      await escape();
      expect(host!.textContent).not.toContain("Discard your entries?");
      expect(field("Project name").value).toBe("Villa");

      await click(button("Cancel"));
      await click(button("Discard"));
      expect(onClose).toHaveBeenCalledTimes(1);
      root.unmount();
    });

    it("ends on a created-not-saved page whose Add devices targets the first line", async () => {
      apiMock.newProject.mockResolvedValue(treeWithLine(42));
      const { root, onCreated, onClose, onAddDevices } = await renderDialog();
      await submitForm();

      expect(onCreated).toHaveBeenCalledTimes(1);
      expect(onClose).not.toHaveBeenCalled();
      expect(host!.querySelector("h2")!.textContent).toBe("Project created");
      expect(host!.textContent).toContain("Untitled project is open. It is not saved yet");
      expect(document.activeElement?.textContent).toBe("Done");

      await click(button("Add devices now"));
      expect(onClose).toHaveBeenCalledTimes(1);
      expect(onAddDevices).toHaveBeenCalledWith(42);
      root.unmount();
    });

    it("shows a server refusal of the structure as an error and stays on the wizard", async () => {
      apiMock.newProject.mockRejectedValueOnce(httpError(422, "areas[0].lines[1]: line address 1 already used"));
      const { root, onCreated } = await renderDialog();
      await submitForm();
      expect(onCreated).not.toHaveBeenCalled();
      expect(host!.textContent).toContain("areas[0].lines[1]: line address 1 already used");
      expect(button("Create project").disabled).toBe(false);
      root.unmount();
    });

    it("speaks German on every step it adds", async () => {
      saveUiLanguage(settingsStorage, "de");
      resetUiLanguageForTests();
      const { root } = await renderDialog();
      await click(button("Weiter"));
      expect(host!.querySelector("h3")!.textContent).toBe("Schritt 2 von 5: Topologie");
      expect(labelled("Bereichsname").map((i) => i.value)).toEqual(["Bereich 1"]);
      expect(labelled("Linienname").map((i) => i.value)).toEqual(["Linie 1.1"]);
      root.unmount();
    });
  });
});
