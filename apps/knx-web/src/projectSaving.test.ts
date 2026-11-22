/** WP-31: projectSaving is recognised in every client error shape and worded per language. */
import { afterEach, describe, expect, it, vi } from "vitest";
import { errorMessage } from "./api";
import { translateFor } from "./i18n";
import { isProjectSaving, projectSavingMessage } from "./projectSaving";

const active = vi.hoisted(() => ({ language: "en" }));
vi.mock("./uiLanguage", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./uiLanguage")>()),
  getActiveUiLanguage: () => active.language,
}));

const withBody = (status: number, kind: string) => Object.assign(new Error("server text"), { status, body: { kind } });
const withKind = (status: number, kind: string) => Object.assign(new Error("server text"), { status, kind });

afterEach(() => { active.language = "en"; });

describe("projectSaving", () => {
  it("is recognised in the api.ts and the feature-client error shapes", () => {
    expect(isProjectSaving(withBody(409, "projectSaving"))).toBe(true);
    expect(isProjectSaving(withKind(409, "projectSaving"))).toBe(true);
  });

  it("is not any other refusal", () => {
    expect(isProjectSaving(withBody(409, "noteStale"))).toBe(false);
    expect(isProjectSaving(withBody(500, "projectSaving"))).toBe(false);
    expect(isProjectSaving(new Error("projectSaving"))).toBe(false);
    expect(isProjectSaving(null)).toBe(false);
    expect(isProjectSaving("projectSaving")).toBe(false);
  });

  it("words the retry in the active language, not with the server's English text", () => {
    expect(projectSavingMessage(withBody(409, "projectSaving"))).toBe(translateFor("en", "projectSaving.retry"));
    active.language = "de";
    const german = projectSavingMessage(withKind(409, "projectSaving"));
    expect(german).toBe(translateFor("de", "projectSaving.retry"));
    expect(german).not.toBe(translateFor("en", "projectSaving.retry"));
    expect(projectSavingMessage(withBody(409, "noteStale"))).toBeNull();
  });

  it("is what the shared errorMessage shows, and other errors keep their text", () => {
    expect(errorMessage(withBody(409, "projectSaving"))).toBe(translateFor("en", "projectSaving.retry"));
    expect(errorMessage(withBody(409, "noteStale"))).toBe("server text");
  });
});
