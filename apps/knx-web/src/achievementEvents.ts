/** What the UI reports to the achievement tracker, and the one channel those reports travel on. */
// ADR-0089. A component that does something worth noticing — saves at
// three in the morning, undoes its hundredth edit — calls
// `emitAchievementEvent` and nothing more. It does not know which
// achievements exist, whether they are switched on, or whether anyone is
// listening: `useAchievements` (App level) is the one subscriber, and with
// nobody subscribed an emit is simply dropped.
//
// Events carry facts, never time: the tracker stamps "now" itself, so a
// rule that cares about the clock (night shift, Christmas) is evaluated
// against one injected clock rather than whatever each emitter read.
//
// Safety boundary (grill-me Q10, ADR-0089): an event names an *outcome*
// the user reached. There is no event for "a telegram was sent" and none
// may be added: achievements reward results, never volume on a live bus.

export type AchievementEvent =
  | { type: "onboardingCompleted" }
  | { type: "projectCreated" }
  | { type: "projectOpened" }
  | { type: "projectSaved" }
  | { type: "autosaveSucceeded" }
  | { type: "undo" }
  | { type: "commandPaletteUsed" }
  | { type: "themeChanged" }
  | { type: "uiLanguageChanged" }
  | { type: "konamiCode" }
  /** The project on screen, measured. Sent whenever the tree changes. */
  | { type: "projectObserved"; groupAddressCount: number };

export type AchievementEventType = AchievementEvent["type"];

type Listener = (event: AchievementEvent) => void;

const listeners = new Set<Listener>();

/** Reports an event to whoever is listening; a no-op when nobody is. */
export function emitAchievementEvent(event: AchievementEvent): void {
  for (const listener of [...listeners]) listener(event);
}

/** Subscribes `listener`; returns the unsubscribe function. */
export function subscribeAchievementEvents(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
