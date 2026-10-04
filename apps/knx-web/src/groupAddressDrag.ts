/** Typed drag payload for linking a group address (UX-01): one decimal id under its own MIME. */

export const GROUP_ADDRESS_DRAG_MIME = "application/x-knxbench-group-address-id";

export function writeDraggedGroupAddress(dataTransfer: DataTransfer, id: number): void {
  dataTransfer.setData(GROUP_ADDRESS_DRAG_MIME, String(id));
  dataTransfer.effectAllowed = "link";
}

// During dragover a browser exposes only the types (the data is protected),
// so a target can accept by type alone; the id is read and checked on drop.
export function carriesGroupAddress(dataTransfer: DataTransfer): boolean {
  return Array.from(dataTransfer.types).includes(GROUP_ADDRESS_DRAG_MIME);
}

export function readDraggedGroupAddress(dataTransfer: DataTransfer): number | null {
  if (!carriesGroupAddress(dataTransfer)) return null;
  const raw = dataTransfer.getData(GROUP_ADDRESS_DRAG_MIME);
  if (!/^[1-9]\d*$/.test(raw)) return null;
  const id = Number(raw);
  return Number.isSafeInteger(id) ? id : null;
}
