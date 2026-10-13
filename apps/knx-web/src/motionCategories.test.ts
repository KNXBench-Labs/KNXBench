/** KL-43: inherited preferences and reduction-only category priority. */
import { expect, it, vi } from "vitest";
import * as motion from "./motion";

it("inherits the old global preference when a category has never been stored", () => {
  const storage = { getItem: vi.fn(() => null), setItem: vi.fn() };
  for (const category of ["toasts", "achievements", "flow"] as const) {
    expect(motion.loadMotionCategory(storage, category)).toBe("inherit");
    expect(motion.effectiveMotionLevel("subtle", "inherit", false)).toBe("subtle");
  }
  expect(storage.setItem).not.toHaveBeenCalled();
});

it("categories may reduce global motion but never bypass Off or the operating system", () => {
  expect(motion.effectiveMotionLevel("standard", "off", false)).toBe("off");
  expect(motion.effectiveMotionLevel("standard", "subtle", false)).toBe("subtle");
  expect(motion.effectiveMotionLevel("off", "inherit", false)).toBe("off");
  expect(motion.effectiveMotionLevel("subtle", "inherit", false)).toBe("subtle");
  for (const global of ["off", "subtle", "standard"]) for (const category of ["inherit", "off", "subtle"]) {
    expect(motion.effectiveMotionLevel(global, category, true)).toBe("off");
  }
  expect(motion.effectiveMotionLevel("standard", "future", false)).toBe("standard");
});

it("round-trips choices independently and keeps unknown stored preferences untouched", () => {
  const values: Record<string, string> = { motionToasts: "future", motionFlow: "off" };
  const storage = { getItem: (key: string) => values[key] ?? null, setItem: (key: string, value: string) => { values[key] = value; } };
  expect(motion.loadMotionCategory(storage, "toasts")).toBe("inherit");
  expect(values.motionToasts).toBe("future");
  motion.saveMotionCategory(storage, "achievements", "subtle");
  expect(motion.loadMotionCategory(storage, "achievements")).toBe("subtle");
  expect(motion.loadMotionCategory(storage, "flow")).toBe("off");
  expect(values.motionToasts).toBe("future");
});
