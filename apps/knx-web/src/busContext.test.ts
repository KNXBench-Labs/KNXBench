/** Tests the project fingerprint and the three-valued lock the companion window depends on. */
// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from "vitest";
import {
  PROJECT_CONTEXT_KEY,
  SESSION_CONTEXT_KEY,
  contextLock,
  fingerprintProjectContext,
  forgetSessionContext,
  projectContextKnown,
  publishProjectContext,
  readContextLock,
  readProjectContext,
  readSessionContext,
  recordSessionContext,
  subscribeContextChanges,
} from "./busContext";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";

function address(overrides: Partial<GroupAddressNode> = {}): GroupAddressNode {
  return {
    id: 1,
    name: "Kitchen ceiling switch",
    address: "1/2/3",
    range: null,
    dpts: ["DPST-1-1"],
    links: [],
    ...overrides,
  };
}

function tree(addresses: GroupAddressNode[]): ProjectTree {
  return {
    schema_version: 3,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    installations: [
      {
        id: 1,
        name: "Installation",
        topology: [],
        buildings: [],
        unassigned: [],
        group_addresses: addresses,
        group_ranges: [],
      },
    ],
  };
}

beforeEach(() => {
  window.localStorage.clear();
});

describe("fingerprintProjectContext", () => {
  it("is stable for an unchanged project", () => {
    expect(fingerprintProjectContext(tree([address()]))).toBe(
      fingerprintProjectContext(tree([address()])),
    );
  });

  // The three facts `GroupAddressContext` freezes, one test each: the
  // formatted address (which carries the project's group-address style),
  // the name, and the DPT set. Each of these changing means a running
  // session decodes, or resolves a write against, something the project no
  // longer says.
  it("changes when a group address is renamed", () => {
    expect(fingerprintProjectContext(tree([address({ name: "Renamed" })]))).not.toBe(
      fingerprintProjectContext(tree([address()])),
    );
  });

  it("changes when a group address's DPT changes", () => {
    expect(fingerprintProjectContext(tree([address({ dpts: ["DPST-5-1"] })]))).not.toBe(
      fingerprintProjectContext(tree([address()])),
    );
  });

  it("changes when the formatted address changes (a different style, or a moved address)", () => {
    expect(fingerprintProjectContext(tree([address({ address: "1/515" })]))).not.toBe(
      fingerprintProjectContext(tree([address()])),
    );
  });

  it("changes when an address is added", () => {
    expect(
      fingerprintProjectContext(tree([address(), address({ id: 2, address: "1/2/4" })])),
    ).not.toBe(fingerprintProjectContext(tree([address()])));
  });

  // The counterweight. A fingerprint that moved on every project edit would
  // lock sessions for reasons that cannot affect a single decoded value,
  // and a lock that cries wolf is a lock users learn to click past.
  it("does not change when something that cannot affect decoding changes", () => {
    const before = tree([address()]);
    const after = tree([address()]);
    after.installations[0].name = "Renamed installation";
    after.warnings = 12;
    after.can_undo = true;
    expect(fingerprintProjectContext(after)).toBe(fingerprintProjectContext(before));
  });

  // The field separators earn their keep here. Without them these two
  // projects flatten to the identical string `1/1/10FooDPST-1-1`, one
  // address each, and the lock reports `"synced"` across a rename that
  // moved a group address — a false negative reachable by ordinary
  // editing, not by 2^-32 luck. This pair was twice reported as a live
  // defect by reviewers who could not see the non-printing separators in
  // the source; it is pinned so that reading the test settles it.
  it("separates address from name, so a shifted boundary is not a collision", () => {
    expect(fingerprintProjectContext(tree([address({ address: "1/1/1", name: "0Foo" })]))).not.toBe(
      fingerprintProjectContext(tree([address({ address: "1/1/10", name: "Foo" })])),
    );
  });

  // The same hazard one level up: the record separator between addresses.
  // Both projects hold two addresses, so the `count-` prefix is identical
  // and cannot be what distinguishes them. The only difference is where a
  // single `1` sits — at the end of the first address's DPT list, or at
  // the start of the second address's address — and without the record
  // separator both flatten to the same bytes.
  it("separates one group address from the next", () => {
    const before = tree([
      address({ dpts: ["D1"] }),
      address({ id: 2, address: "/1/2", name: "Y", dpts: [] }),
    ]);
    const after = tree([
      address({ dpts: ["D"] }),
      address({ id: 2, address: "1/1/2", name: "Y", dpts: [] }),
    ]);
    expect(fingerprintProjectContext(before)).not.toBe(fingerprintProjectContext(after));
  });

  it("fingerprints an absent project as a value rather than throwing", () => {
    expect(fingerprintProjectContext(null)).toBe("none");
  });
});

describe("contextLock", () => {
  const project = { fingerprint: "abc", at: 2_000 };

  it("is synced with no session attached", () => {
    expect(contextLock(project, null, null)).toBe("synced");
  });

  it("is synced while the fingerprints match", () => {
    expect(contextLock(project, { sessionId: 4, fingerprint: "abc", at: 1_000 }, 4)).toBe("synced");
  });

  it("is stale once the project fingerprint has moved", () => {
    expect(contextLock(project, { sessionId: 4, fingerprint: "xyz", at: 1_000 }, 4)).toBe("stale");
  });

  it("is unverified when no window recorded this session's start", () => {
    expect(contextLock(project, null, 4)).toBe("unverified");
  });

  // The connection-state case: the session on the wire is not the one whose
  // fingerprint was recorded, so the recorded fingerprint says nothing
  // about it. Silence would be the wrong answer here — this is precisely
  // the "a different session started under you" situation.
  it("is unverified when the recorded session is a different one", () => {
    expect(contextLock(project, { sessionId: 3, fingerprint: "abc", at: 1_000 }, 4)).toBe(
      "unverified",
    );
  });

  it("is synced when there was no project then and none now", () => {
    expect(contextLock(null, { sessionId: 4, fingerprint: null, at: 1_000 }, 4)).toBe("synced");
  });

  // A session started with no project open freezes an empty context: no
  // names, no DPTs, no style. Opening a project afterwards does not
  // retrofit any of that into the running session, so the monitor is now
  // showing raw addresses for a project that has names — stale, not merely
  // unverified.
  it("is stale when a project appeared after a session started without one", () => {
    expect(contextLock(project, { sessionId: 4, fingerprint: null, at: 1_000 }, 4)).toBe("stale");
  });

  it("is unverified when the project record vanished under a session that had one", () => {
    expect(contextLock(null, { sessionId: 4, fingerprint: "abc", at: 1_000 }, 4)).toBe(
      "unverified",
    );
  });
});

describe("records in localStorage", () => {
  it("round-trips a published project and a recorded session", () => {
    publishProjectContext(tree([address()]));
    recordSessionContext(9);

    const published = readProjectContext(window.localStorage);
    const recorded = readSessionContext(window.localStorage);
    expect(published?.fingerprint).toBe(fingerprintProjectContext(tree([address()])));
    expect(recorded?.sessionId).toBe(9);
    expect(recorded?.fingerprint).toBe(published?.fingerprint);
    expect(readContextLock(9)).toBe("synced");
  });

  // The whole point, end to end and without a server: connect, edit, and
  // the lock closes.
  it("goes stale when the project is republished after the session was recorded", () => {
    publishProjectContext(tree([address()]));
    recordSessionContext(9);
    publishProjectContext(tree([address({ name: "Renamed after connecting" })]));
    expect(readContextLock(9)).toBe("stale");
  });

  it("forgets the session record on disconnect", () => {
    publishProjectContext(tree([address()]));
    recordSessionContext(9);
    forgetSessionContext();
    expect(readSessionContext(window.localStorage)).toBeNull();
    expect(readContextLock(9)).toBe("unverified");
  });

  it("treats a corrupt record as absent rather than trusting it", () => {
    window.localStorage.setItem(PROJECT_CONTEXT_KEY, "{not json");
    window.localStorage.setItem(SESSION_CONTEXT_KEY, '{"sessionId":"nine"}');
    expect(readProjectContext(window.localStorage)).toBeNull();
    expect(readSessionContext(window.localStorage)).toBeNull();
  });

  it("knows whether a project has been published", () => {
    expect(projectContextKnown()).toBe(false);
    publishProjectContext(tree([address()]));
    expect(projectContextKnown()).toBe(true);
  });
});

describe("subscribeContextChanges", () => {
  // The `storage` event does not fire in the window that wrote the value,
  // so a same-window notification is not a nicety: without it a window
  // holding both the editor and a monitor would never hear its own
  // publication.
  it("notifies the publishing window itself", () => {
    let calls = 0;
    const unsubscribe = subscribeContextChanges(() => {
      calls += 1;
    });
    publishProjectContext(tree([address()]));
    expect(calls).toBe(1);
    unsubscribe();
    publishProjectContext(tree([address({ name: "Again" })]));
    expect(calls).toBe(1);
  });

  it("notifies on a cross-window storage event for either record", () => {
    let calls = 0;
    const unsubscribe = subscribeContextChanges(() => {
      calls += 1;
    });
    window.dispatchEvent(new StorageEvent("storage", { key: PROJECT_CONTEXT_KEY }));
    window.dispatchEvent(new StorageEvent("storage", { key: SESSION_CONTEXT_KEY }));
    window.dispatchEvent(new StorageEvent("storage", { key: "knx-desktop:theme" }));
    expect(calls).toBe(2);
    unsubscribe();
  });
});
