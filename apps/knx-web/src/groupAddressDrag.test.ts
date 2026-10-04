/** Tests for the group-address drag payload: typed MIME, strict id, foreign data refused. */
import { describe, expect, it } from "vitest";
import {
  GROUP_ADDRESS_DRAG_MIME,
  carriesGroupAddress,
  readDraggedGroupAddress,
  writeDraggedGroupAddress,
} from "./groupAddressDrag";

class Transfer {
  private readonly values = new Map<string, string>();
  effectAllowed = "uninitialized";
  get types(): string[] { return [...this.values.keys()]; }
  setData(format: string, data: string) { this.values.set(format, data); }
  getData(format: string) { return this.values.get(format) ?? ""; }
}

describe("group-address drag payload", () => {
  it("writes only the typed decimal id and allows linking", () => {
    const transfer = new Transfer();
    writeDraggedGroupAddress(transfer as unknown as DataTransfer, 9);
    expect(transfer.types).toEqual([GROUP_ADDRESS_DRAG_MIME]);
    expect(transfer.getData(GROUP_ADDRESS_DRAG_MIME)).toBe("9");
    expect(transfer.effectAllowed).toBe("link");
    expect(carriesGroupAddress(transfer as unknown as DataTransfer)).toBe(true);
    expect(readDraggedGroupAddress(transfer as unknown as DataTransfer)).toBe(9);
  });

  it.each(["", "0", "-1", "9.5", "09", "1e3", " 9", "x"])("refuses the payload %j", (raw) => {
    const transfer = new Transfer();
    transfer.setData(GROUP_ADDRESS_DRAG_MIME, raw);
    expect(readDraggedGroupAddress(transfer as unknown as DataTransfer)).toBeNull();
  });

  it("ignores foreign drag data such as a device or plain text", () => {
    const transfer = new Transfer();
    transfer.setData("application/x-knxbench-device-id", "9");
    transfer.setData("text/plain", "9");
    expect(carriesGroupAddress(transfer as unknown as DataTransfer)).toBe(false);
    expect(readDraggedGroupAddress(transfer as unknown as DataTransfer)).toBeNull();
  });
});
