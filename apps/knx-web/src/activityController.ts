/** Tracks session activities by producer identity without executing or cancelling their work. */
import type { MessageKey } from "./i18n";
export const ACTIVITY_DELAY_MS = 2_000;
export const ACTIVITY_SUCCESS_MS = 3_000;
export const ACTIVITY_RECENT_LIMIT = 20;
export type ActivityKind = "open" | "import" | "resume" | "save" | "autosave" | "deviceDetail" | "catalogue" | "compare";
export type ActivityState = "running" | "waiting" | "cancelling" | "uncertain" | "succeeded" | "failed" | "cancelled" | "discarded" | "decision";
export type ActivityScope = object;
export interface ActivityIdentity {
  kind: ActivityKind;
  operationId: string;
  projectScope?: ActivityScope;
  entityId?: string;
  access: "read" | "write" | "independent";
}
export interface ActivityContext {
  readonly projectName?: string;
  readonly entities?: readonly { readonly label: string; readonly address: string | null }[];
}
export function projectActivityContext(tree: { installations: readonly { name: string }[] } | null): ActivityContext {
  return { projectName: tree?.installations.map(installation => installation.name).join(" / ") || undefined };
}
export interface ActivityEntry extends ActivityContext {
  readonly id: number;
  readonly identity: ActivityIdentity;
  readonly source?: string;
  readonly state: ActivityState;
  readonly phase?: string;
  readonly phaseKey?: MessageKey;
  readonly errorKey?: MessageKey;
  readonly resultKey?: MessageKey;
  readonly progress?: { completed: string; total: string };
  readonly error?: string;
  readonly destination?: "catalogue" | "comparison";
  readonly startedAt: number;
  readonly finishedAt?: number;
  readonly finishedOrder?: number;
  readonly visible: boolean;
  readonly acknowledged: boolean;
  readonly highlightUntil?: number;
}
export type ActivityUpdate = Pick<Partial<ActivityEntry>, "phase" | "phaseKey" | "progress" | "error" | "errorKey" | "projectName" | "resultKey"> & {
  state?: "running" | "waiting" | "cancelling" | "uncertain";
};
export interface ActivityHandle {
  readonly id: number;
  update(update: ActivityUpdate): boolean;
  finish(state?: "succeeded" | "failed" | "cancelled" | "discarded" | "decision", error?: string): boolean;
}
export interface ActivityClock {
  now(): number;
  schedule(callback: () => void, milliseconds: number): unknown;
  clear(timer: unknown): void;
}
const SYSTEM_CLOCK: ActivityClock = {
  now: () => Date.now(),
  schedule: (callback, ms) => setTimeout(callback, ms),
  clear: timer => clearTimeout(timer as ReturnType<typeof setTimeout>),
};
export function activityTerminal(state: ActivityState): boolean {
  return state === "succeeded" || state === "failed" || state === "cancelled" || state === "discarded" || state === "decision";
}
export function activityHighlighted(entry: ActivityEntry): boolean {
  return entry.visible && (!activityTerminal(entry.state) || entry.state === "failed" && !entry.acknowledged || entry.highlightUntil !== undefined);
}

export class ActivityController {
  #entries: readonly ActivityEntry[] = [];
  #listeners = new Set<() => void>();
  #nextId = 0;
  #nextFinishedOrder = 0;
  #timer: unknown = null;
  #connections = 0;
  constructor(private readonly clock: ActivityClock = SYSTEM_CLOCK) {}
  getSnapshot = () => this.#entries;
  subscribe = (listener: () => void) => {
    this.#listeners.add(listener);
    return () => { this.#listeners.delete(listener); };
  };
  // Disconnecting only releases the presentation timer; it never asserts worker cancellation.
  connect = () => {
    this.#connections += 1;
    this.#tick();
    return () => {
      this.#connections -= 1;
      if (this.#connections === 0) this.#clearTimer();
    };
  };
  begin(identity: ActivityIdentity, options: ActivityContext & { source?: string; destination?: "catalogue" | "comparison" } = {}): ActivityHandle {
    const existing = this.#entries.find(row => row.identity.kind === identity.kind && row.identity.operationId === identity.operationId
      && row.identity.projectScope === identity.projectScope && row.identity.entityId === identity.entityId);
    if (existing) return this.#handle(existing.id);
    const id = ++this.#nextId;
    const entry: ActivityEntry = {
      id, identity: { ...identity }, ...options,
      entities: options.entities?.map(entity => ({ ...entity })), state: "running", startedAt: this.clock.now(),
      visible: false, acknowledged: false,
    };
    this.#publish([...this.#entries, entry]);
    return this.#handle(id);
  }
  #handle(id: number): ActivityHandle {
    // The closure, not a filename, entity ID or mutable current-operation ref, authorizes updates.
    return {
      id,
      update: update => this.#change(id, previous => ({ ...previous, ...update })),
      finish: (state = "succeeded", error) => this.#change(id, previous => ({
        ...previous, state, error: error ?? previous.error, finishedAt: this.clock.now(), finishedOrder: ++this.#nextFinishedOrder,
        visible: state === "failed" || previous.visible,
        highlightUntil: (state === "succeeded" || state === "decision") && previous.visible ? this.clock.now() + ACTIVITY_SUCCESS_MS : undefined,
        phase: undefined, phaseKey: undefined, progress: undefined,
      })),
    };
  }
  acknowledge(id: number): void {
    this.#publish(this.#entries.map(entry => entry.id === id && entry.state === "failed"
      ? { ...entry, acknowledged: true } : entry));
  }
  invalidateScope(scope: ActivityScope): void {
    // A stale read is discarded locally. Writes retain their actual owner until its response settles.
    this.#publish(this.#entries.map(entry => entry.identity.projectScope === scope
      && entry.identity.access === "read" && !activityTerminal(entry.state)
      ? { ...entry, state: "discarded", finishedAt: this.clock.now(), finishedOrder: ++this.#nextFinishedOrder, phase: undefined, phaseKey: undefined, progress: undefined } : entry));
  }
  #change(id: number, change: (entry: ActivityEntry) => ActivityEntry): boolean {
    const entry = this.#entries.find(row => row.id === id);
    if (!entry || activityTerminal(entry.state)) return false;
    this.#publish(this.#entries.map(row => row === entry ? change(row) : row));
    return true;
  }
  #publish(entries: readonly ActivityEntry[]): void {
    // Unacknowledged errors are outstanding attention, not evictable recent history.
    const recent = entries.filter(row => activityTerminal(row.state) && !(row.state === "failed" && !row.acknowledged));
    const keep = new Set(recent.sort((a, b) => (a.finishedOrder ?? 0) - (b.finishedOrder ?? 0)).slice(-ACTIVITY_RECENT_LIMIT).map(row => row.id));
    this.#entries = entries.filter(row => !activityTerminal(row.state) || row.state === "failed" && !row.acknowledged || keep.has(row.id));
    this.#listeners.forEach(listener => listener());
    this.#schedule();
  }
  #clearTimer(): void {
    if (this.#timer !== null) this.clock.clear(this.#timer);
    this.#timer = null;
  }
  #schedule(): void {
    this.#clearTimer();
    if (this.#connections === 0) return;
    const now = this.clock.now();
    const deadlines = this.#entries.flatMap(entry => !activityTerminal(entry.state) && !entry.visible
      ? [entry.startedAt + ACTIVITY_DELAY_MS]
      : entry.highlightUntil !== undefined ? [entry.highlightUntil] : []);
    if (deadlines.length) this.#timer = this.clock.schedule(() => { this.#timer = null; this.#tick(); }, Math.max(0, Math.min(...deadlines) - now));
  }
  #tick(): void {
    const now = this.clock.now();
    this.#publish(this.#entries.map(entry => ({ ...entry,
      visible: entry.visible || !activityTerminal(entry.state) && now >= entry.startedAt + ACTIVITY_DELAY_MS,
      highlightUntil: entry.highlightUntil !== undefined && now >= entry.highlightUntil ? undefined : entry.highlightUntil,
    })));
  }
}
