/** A consistent small outline icon vocabulary, rendered without an icon dependency. */
export type WorkbenchIconName =
  | "overview" | "buildings" | "topology" | "addresses" | "catalog" | "log" | "monitor" | "panel"
  // Achievement glyphs (ADR-0089), drawn in the same 24-unit outline style.
  | "trophy" | "star" | "command" | "palette" | "language" | "save" | "undo" | "moon" | "gift" | "gamepad";
const paths: Record<WorkbenchIconName, string> = {
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  buildings: "M3 10l9-7 9 7 M5 9v12h14V9 M9 21v-8h6v8",
  topology: "M9 2h6v5H9z M2 17h6v5H2z M16 17h6v5h-6z M12 7v5 M5 17v-5h14v5",
  addresses: "M8 5h13 M8 12h13 M8 19h13 M3 5h.01 M3 12h.01 M3 19h.01",
  catalog: "M3 7l9-4 9 4v13H3z M3 7h18 M9 7v13 M15 7v13",
  log: "M5 2h10l4 4v16H5z M14 2v5h5 M8 12h8 M8 16h8",
  monitor: "M2 12h4l3-8 5 16 3-8h5",
  panel: "M3 4h18v16H3z M9 4v16",
  trophy: "M7 4h10v5a5 5 0 01-10 0z M7 6H4a3 3 0 003 4 M17 6h3a3 3 0 01-3 4 M12 14v4 M8 21h8 M9 18h6",
  star: "M12 3l2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.3l-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z",
  command: "M9 9h6v6H9z M9 9V6a3 3 0 10-3 3h3 M15 9V6a3 3 0 113 3h-3 M9 15v3a3 3 0 11-3-3h3 M15 15v3a3 3 0 103-3h-3",
  palette: "M12 3a9 9 0 100 18c1.5 0 2-1 2-2s-1-1.5-1-2.5 1-1.5 2-1.5h2a4 4 0 004-4c0-4.4-4-8-9-8z M7.5 11h.01 M10 7h.01 M15 7h.01",
  language: "M3 5h12 M9 3v2 M5 5c1 4 4 7 8 9 M13 5c-1 4-4 7-8 9 M13 21l4-9 4 9 M14.5 18h5",
  save: "M5 3h11l3 3v15H5z M8 3v5h7V3 M8 21v-7h8v7",
  undo: "M8 4L3 9l5 5 M3 9h10a6 6 0 010 12",
  moon: "M20 14.5A8 8 0 019.5 4a8 8 0 1010.5 10.5z",
  gift: "M3 8h18v4H3z M5 12v9h14v-9 M12 8v13 M12 8c-2-4-6-4-6-1s4 1 6 1 M12 8c2-4 6-4 6-1s-4 1-6 1",
  gamepad: "M6 8h12a4 4 0 014 4v2a3 3 0 01-5.2 2L15 14H9l-1.8 2A3 3 0 012 14v-2a4 4 0 014-4z M7 11v3 M5.5 12.5h3 M15 12h.01 M18 13h.01",
};
export default function WorkbenchIcon({ name }: { name: WorkbenchIconName }) {
  return <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
