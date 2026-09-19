/** The help system's non-visual half: its topic list, its shortcut rule and its motion guard. */
// ADR-0024. Nothing here renders anything and nothing here holds prose:
// every user-facing word is a `MessageKey` resolved through `i18n.ts` at
// render time, so a topic that names a key the catalogue does not have is
// a compile error rather than a blank paragraph, and the German half is a
// compile error too (`messages/de.ts` is `Record<MessageKey, string>`).

import type { MessageKey } from "./i18n";

/**
 * One entry in the help panel's topic list.
 *
 * `bodyKeys` is a list because a catalogue value is one paragraph of
 * plain text and the catalogue holds no markup (ADR-0024, decision 2).
 * Paragraph order lives here, in code, rather than inside a string that
 * a translator could reflow.
 *
 * `standardNote` appends the shared "this describes how KNXBench uses the
 * concept, it is not a substitute for the KNX Standard" sentence. It is a
 * flag rather than a copied final paragraph so the sentence is written and
 * translated exactly once — and so it cannot drift between the four topics
 * that carry it.
 */
export interface HelpTopic {
  id: string;
  titleKey: MessageKey;
  bodyKeys: readonly MessageKey[];
  standardNote: boolean;
}

/**
 * Every help topic, in the order the panel lists them.
 *
 * Ordered by what a person meets first — how to get a project open, then
 * the window, then the structures inside it, then the bus, then the
 * boundaries of what this application actually does. The last topic is
 * deliberately the limitations one: a help system that only describes
 * what works is a sales brochure.
 *
 * `satisfies readonly HelpTopic[]` rather than a type annotation so the
 * literal keeps its narrow key types for the `MessageKey` check while
 * still being checked against the interface.
 */
export const HELP_TOPICS = [
  {
    id: "gettingStarted",
    titleKey: "help.topic.gettingStarted.title",
    bodyKeys: [
      "help.topic.gettingStarted.p1",
      "help.topic.gettingStarted.p2",
      "help.topic.gettingStarted.p3",
    ],
    standardNote: false,
  },
  {
    id: "workbench",
    titleKey: "help.topic.workbench.title",
    bodyKeys: ["help.topic.workbench.p1", "help.topic.workbench.p2", "help.topic.workbench.p3"],
    standardNote: false,
  },
  {
    id: "buildings",
    titleKey: "help.topic.buildings.title",
    bodyKeys: ["help.topic.buildings.p1", "help.topic.buildings.p2"],
    standardNote: true,
  },
  {
    id: "topology",
    titleKey: "help.topic.topology.title",
    bodyKeys: ["help.topic.topology.p1", "help.topic.topology.p2", "help.topic.topology.p3"],
    standardNote: true,
  },
  {
    id: "groupAddresses",
    titleKey: "help.topic.groupAddresses.title",
    bodyKeys: [
      "help.topic.groupAddresses.p1",
      "help.topic.groupAddresses.p2",
      "help.topic.groupAddresses.p3",
    ],
    standardNote: true,
  },
  {
    id: "comObjectFlags",
    titleKey: "help.topic.comObjectFlags.title",
    bodyKeys: [
      "help.topic.comObjectFlags.p1",
      "help.topic.comObjectFlags.p2",
      "help.topic.comObjectFlags.p3",
    ],
    standardNote: true,
  },
  {
    id: "busMonitor",
    titleKey: "help.topic.busMonitor.title",
    bodyKeys: ["help.topic.busMonitor.p1", "help.topic.busMonitor.p2", "help.topic.busMonitor.p3"],
    standardNote: false,
  },
  {
    id: "importExport",
    titleKey: "help.topic.importExport.title",
    bodyKeys: [
      "help.topic.importExport.p1",
      "help.topic.importExport.p2",
      "help.topic.importExport.p3",
    ],
    standardNote: false,
  },
  {
    id: "keyboard",
    titleKey: "help.topic.keyboard.title",
    bodyKeys: ["help.topic.keyboard.p1", "help.topic.keyboard.p2", "help.topic.keyboard.p3"],
    standardNote: false,
  },
  {
    id: "limits",
    titleKey: "help.topic.limits.title",
    bodyKeys: ["help.topic.limits.p1", "help.topic.limits.p2", "help.topic.limits.p3"],
    standardNote: false,
  },
] as const satisfies readonly HelpTopic[];

/** The shared closing sentence `HelpTopic.standardNote` opts a topic into. */
export const HELP_STANDARD_NOTE_KEY = "help.standardNote" satisfies MessageKey;

/** The topic the panel opens on when nothing else is chosen. */
export const DEFAULT_HELP_TOPIC_ID = HELP_TOPICS[0].id;

/** The paragraph keys of `id`, with the shared standard note appended when
 * the topic asks for it — the one place that expansion happens, so the
 * panel and its tests read the same list. `undefined` for an unknown id,
 * which is a caller bug rather than a state the UI can reach. */
export function helpTopicParagraphs(id: string): readonly MessageKey[] | undefined {
  const topic = HELP_TOPICS.find((t) => t.id === id);
  if (!topic) return undefined;
  return topic.standardNote ? [...topic.bodyKeys, HELP_STANDARD_NOTE_KEY] : topic.bodyKeys;
}

/**
 * Whether a key event is the "open help" shortcut.
 *
 * Plain `F1`, no modifier. A modified `F1` belongs to the browser and the
 * window manager — Ctrl+F1 and Alt+F1 are bound in several Linux desktops
 * — and silently swallowing those would be this application taking a key
 * it was never given. Stated as a pure predicate so the rule is testable
 * without a DOM, and so the mutation that loosens it fails a test rather
 * than a click-through.
 */
export function opensHelp(
  e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">,
): boolean {
  return e.key === "F1" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey;
}

/**
 * Whether a help tip's bubble may fade in, or must simply be there.
 *
 * Exactly the shape and exactly the reading of `loadFlavour.ts`'s
 * `flavourRotates`: the application's own `data-motion-level="off"` wins
 * outright, the operating system's `prefers-reduced-motion: reduce` wins
 * next, and a runtime with no `matchMedia` at all has not asked for
 * stillness — an absent API is an absent API, not a preference. Under
 * either switch the bubble still appears; it just arrives instead of
 * animating, because someone who asked for calm asked for less movement,
 * not for less help.
 */
export function helpTipAnimates(
  win: Pick<Window, "matchMedia">,
  root: Pick<Element, "getAttribute">,
): boolean {
  if (root.getAttribute("data-motion-level") === "off") return false;
  if (typeof win.matchMedia !== "function") return true;
  return !win.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
