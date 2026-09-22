/** Verifies line-scan exclusions preserve legacy text while validating new addresses strictly. */
// @vitest-environment happy-dom
import { afterEach, expect, it } from "vitest";
import {
  loadLineScanExclusions,
  saveLineScanExclusions,
  validateIndividualAddress,
} from "./lineScanExclusions";
import { resetSettingsForTests } from "./settingsStore";

afterEach(() => resetSettingsForTests());

it("loads every legacy string byte-for-byte and in order", () => {
  const values = ["2.3.42", "2.3.42", " 2.3.43 ", "16.1.1", "not an address"];
  saveLineScanExclusions(values);
  expect(loadLineScanExclusions()).toEqual(values);
});

it.each([
  ["0.0.0", true],
  ["15.15.255", true],
  ["16.0.1", false],
  ["1.16.1", false],
  ["1.1.256", false],
  [" 1.1.1 ", false],
  ["1/1/1", false],
  ["anything", false],
])("validates complete dotted individual address %s", (value, valid) => {
  expect(validateIndividualAddress(value)).toBe(valid);
});
