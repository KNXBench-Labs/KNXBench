/** A consistent small outline icon vocabulary, rendered without an icon dependency. */
export type WorkbenchIconName = "overview" | "buildings" | "topology" | "addresses" | "catalog" | "log" | "monitor" | "panel";
const paths: Record<WorkbenchIconName, string> = {
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  buildings: "M3 10l9-7 9 7 M5 9v12h14V9 M9 21v-8h6v8",
  topology: "M9 2h6v5H9z M2 17h6v5H2z M16 17h6v5h-6z M12 7v5 M5 17v-5h14v5",
  addresses: "M8 5h13 M8 12h13 M8 19h13 M3 5h.01 M3 12h.01 M3 19h.01",
  catalog: "M3 7l9-4 9 4v13H3z M3 7h18 M9 7v13 M15 7v13",
  log: "M5 2h10l4 4v16H5z M14 2v5h5 M8 12h8 M8 16h8",
  monitor: "M2 12h4l3-8 5 16 3-8h5",
  panel: "M3 4h18v16H3z M9 4v16",
};
export default function WorkbenchIcon({ name }: { name: WorkbenchIconName }) {
  return <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
