import { describe, expect, it } from "vitest";
import { loadTheme, nextTheme, saveTheme } from "./theme";

function fakeStorage(initial: Record<string, string> = {}) {
  const store = { ...initial };
  return {
    getItem: (key: string) => (key in store ? store[key] : null),
    setItem: (key: string, value: string) => {
      store[key] = value;
    },
    _store: store,
  };
}

describe("nextTheme", () => {
  it("cycles system -> light -> dark -> system", () => {
    expect(nextTheme("system")).toBe("light");
    expect(nextTheme("light")).toBe("dark");
    expect(nextTheme("dark")).toBe("system");
  });
});

describe("loadTheme", () => {
  it("resolves to system when nothing is stored", () => {
    expect(loadTheme(fakeStorage())).toBe("system");
  });

  it("resolves to the stored value for light or dark", () => {
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "light" }))).toBe("light");
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "dark" }))).toBe("dark");
  });

  it("resolves to system for an unrecognized stored value", () => {
    expect(loadTheme(fakeStorage({ "knx-desktop:theme": "solarized" }))).toBe("system");
  });
});

describe("saveTheme", () => {
  it("writes the theme under the expected key", () => {
    const storage = fakeStorage();
    saveTheme(storage, "dark");
    expect(storage._store["knx-desktop:theme"]).toBe("dark");
  });
});
