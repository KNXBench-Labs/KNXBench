/** Issue #20: group-address arithmetic and the range's next free address. */
import { describe, expect, it } from "vitest";
import { formatGroupAddressRaw, groupAddressRaw, nextFreeGroupAddress } from "./nextFreeGroupAddress";

const range = (start: string, end: string) => ({ start, end });

describe("groupAddressRaw / formatGroupAddressRaw", () => {
  it.each([
    ["ThreeLevel", "1/2/3", (1 << 11) | (2 << 8) | 3],
    ["ThreeLevel", "31/7/255", 0xffff],
    ["TwoLevel", "1/515", (1 << 11) | 515],
    ["Free", "2563", 2563],
  ])("round-trips %s %s", (style, text, raw) => {
    expect(groupAddressRaw(text, style)).toBe(raw);
    expect(formatGroupAddressRaw(raw, style)).toBe(text);
  });

  it.each([
    ["ThreeLevel", "1/8/0"], ["ThreeLevel", "32/0/0"], ["ThreeLevel", "1/2"], ["ThreeLevel", "1/2/x"],
    ["TwoLevel", "1/2048"], ["Free", "65536"], ["Free", "-1"], ["Unknown", "1/2/3"],
  ])("refuses %s %s", (style, text) => {
    expect(groupAddressRaw(text, style)).toBeNull();
  });
});

describe("nextFreeGroupAddress (issue #20)", () => {
  const style = "ThreeLevel";

  it("takes the first gap in the range's sequence", () => {
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/255"), ["1/0/10", "1/0/11", "1/0/13", "1/0/14"], style))
      .toBe("1/0/12");
  });

  it("does not count the free space before the sequence as a gap", () => {
    // 1/0/0–1/0/9 are free, but the sequence starts at 1/0/10.
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/255"), ["1/0/10", "1/0/11"], style)).toBe("1/0/12");
  });

  it("continues after the last address when there is no gap", () => {
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/255"), ["1/0/1", "1/0/2", "1/0/3"], style)).toBe("1/0/4");
  });

  it("starts an empty range at its first address", () => {
    expect(nextFreeGroupAddress(range("2/1/0", "2/1/255"), [], style)).toBe("2/1/0");
  });

  it("never suggests 0/0/0, the KNX broadcast destination", () => {
    expect(nextFreeGroupAddress(range("0/0/0", "0/0/255"), [], style)).toBe("0/0/1");
    expect(nextFreeGroupAddress(range("0/0/0", "0/0/255"), ["0/0/0", "0/0/2"], style)).toBe("0/0/1");
  });

  it("falls back to the lowest free address when the end of the range is reached", () => {
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/3"), ["1/0/2", "1/0/3"], style)).toBe("1/0/0");
  });

  it("answers null for a full range", () => {
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/2"), ["1/0/0", "1/0/1", "1/0/2"], style)).toBeNull();
  });

  it("treats every address of the installation inside the range as taken, ranged or not", () => {
    // 1/0/12 exists without a range (or in another one); it is still taken.
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/255"), ["1/0/10", "1/0/11", "1/0/13", "1/0/12"], style))
      .toBe("1/0/14");
  });

  it("ignores addresses outside the range and unreadable ones", () => {
    expect(nextFreeGroupAddress(range("1/0/0", "1/0/255"), ["1/1/0", "2/0/5", "garbage", "1/0/0"], style))
      .toBe("1/0/1");
  });

  it("works for two-level and free styles", () => {
    expect(nextFreeGroupAddress(range("3/0", "3/2047"), ["3/0", "3/1", "3/3"], "TwoLevel")).toBe("3/2");
    expect(nextFreeGroupAddress(range("100", "200"), ["100", "101"], "Free")).toBe("102");
  });

  it("answers null for an unreadable or inverted range", () => {
    expect(nextFreeGroupAddress(range("1/0/9", "1/0/1"), [], style)).toBeNull();
    expect(nextFreeGroupAddress(range("x", "1/0/1"), [], style)).toBeNull();
  });
});
