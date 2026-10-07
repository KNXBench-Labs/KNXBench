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
  /** `themeId` is the theme now in use, built in or from a pack. */
  | { type: "themeChanged"; themeId: string }
  | { type: "uiLanguageChanged" }
  | { type: "konamiCode" }
  /** The project on screen, measured (`achievementObservation.ts`). Sent
   * whenever the tree changes. */
  | {
      type: "projectObserved";
      groupAddressCount: number;
      /** Group addresses whose name is not blank. */
      namedGroupAddressCount: number;
      /** Distinct DPT references across all group addresses. */
      distinctDptCount: number;
      /** Building parts of kind `Room`, at any depth. */
      roomCount: number;
    }
  /** An ETS archive became the open project. The counts are the import
   * report's own (`ProjectTree.errors` / `.warnings`): `lostItems` is
   * genuine loss, `notices` everything worth a look. */
  | { type: "etsImported"; lostItems: number; notices: number; deviceCount: number; passwordProtected: boolean }
  | { type: "projectDiffed" }
  | { type: "groupAddressCsvExported" }
  /** Only an import that was applied, not one awaiting confirmation. */
  | { type: "groupAddressCsvImported" }
  | { type: "documentationExported" }
  /** A drag-and-drop that changed the project (move or link). */
  | { type: "dragDropApplied" }
  /** A bulk action the server applied, with the number of items. */
  | { type: "bulkActionApplied"; itemCount: number }
  /** A read-only bus monitor session is open on a KNXnet/IP interface. */
  | { type: "busMonitorStarted" }
  /** One more minute of an open bus monitor session. */
  | { type: "busMonitorMinute" }
  | { type: "lineScanCompleted" }
  | { type: "busCaptureExported" }
  /** The flow view is on screen, showing `telegramCount` admitted telegrams. */
  | { type: "flowWatched"; telegramCount: number }
  /** The offline readiness check graded the project's devices. */
  | { type: "readinessChecked"; deviceCount: number; unplannableCount: number }
  /** A read-only comparison with a real device that passed its consistency checks. */
  | { type: "deviceCompared"; differingOctets: number }
  /** Address programming finished and the device answers at the new address. */
  | { type: "individualAddressVerified" }
  /** A download finished with every block read back unchanged; `subject`
   * is the device's individual address. */
  | { type: "deviceDownloadVerified"; subject: string }
  | { type: "debugReportCreated" }
  /** An error toast was shown (every one wears a joke, `humorizeError`). */
  | { type: "errorToastShown" };

export type AchievementEventType = AchievementEvent["type"];

/** The event of type `T`, payload included. */
export type AchievementEventOf<T extends AchievementEventType> = Extract<AchievementEvent, { type: T }>;

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
