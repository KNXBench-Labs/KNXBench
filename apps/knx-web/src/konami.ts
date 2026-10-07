/** Detects the Konami code typed at the workbench, never inside a text field. */
// ADR-0089's one legendary easter egg. Up, up, down, down, left, right,
// left, right, B, A. The guard matters more than the sequence: arrow keys
// and letters are what people type into tables, search boxes and address
// fields all day, so a keystroke whose target is anything editable never
// reaches the matcher at all.

export const KONAMI_SEQUENCE: readonly string[] = [
  "ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown",
  "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight",
  "b", "a",
];

/** A stateful matcher: feed it keys, it answers `true` on the key that
 * completes the sequence and then starts over. It keeps the last few keys
 * rather than a position, so a stutter ("up, up, up, down…") still counts. */
export function createKonamiMatcher(): (key: string) => boolean {
  const recent: string[] = [];
  return (key: string) => {
    recent.push(key.length === 1 ? key.toLowerCase() : key);
    if (recent.length > KONAMI_SEQUENCE.length) recent.shift();
    const matched =
      recent.length === KONAMI_SEQUENCE.length && recent.every((k, i) => k === KONAMI_SEQUENCE[i]);
    if (matched) recent.length = 0;
    return matched;
  };
}

/** Whether a keystroke aimed at `target` is someone typing. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!target || typeof (target as Element).tagName !== "string") return false;
  const element = target as HTMLElement;
  const tag = element.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || element.isContentEditable === true
    || element.getAttribute("contenteditable") === "true";
}

/** Listens on `target` for the sequence; returns the cleanup function. */
export function installKonamiListener(target: Document, onMatch: () => void): () => void {
  const matcher = createKonamiMatcher();
  function handleKeyDown(event: KeyboardEvent) {
    if (isTypingTarget(event.target)) return;
    if (matcher(event.key)) onMatch();
  }
  target.addEventListener("keydown", handleKeyDown);
  return () => target.removeEventListener("keydown", handleKeyDown);
}
