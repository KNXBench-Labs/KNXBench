/** Mounted online catalogue regression tests using fictional own-backend responses only. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import OnlineCatalogue from "./OnlineCatalogue";
import OnlineCatalogueSettings, {
  OnlineCatalogueAccount,
} from "./OnlineCatalogueAccount";
import ProductInstallControl from "./ProductInstallControl";
import { onlineEntry, onlineStatus } from "./onlineCatalogueFixtures";
import { ONLINE_BASE as base, type OnlineStatus } from "./onlineCatalogueApi";
import { CatalogueJobsProvider } from "./CatalogueJobsContext";
import { CatalogueJobsController } from "./catalogueJobsController";
import { fakeApi, status as jobStatus } from "./catalogueJobsFixtures";
import { messages as en } from "./messages/en";
import type { CatalogueJobsApi } from "./catalogueJobsApi";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

let host: HTMLDivElement;
let root: Root;
let state: OnlineStatus;
let calls: Array<{ path: string; method: string; body: unknown }>;
let fetcher: ReturnType<
  typeof vi.fn<(path: string, init?: RequestInit) => Promise<Response>>
>;

const respond = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "Content-Type": "application/json", "Cache-Control": "no-store" },
  });

beforeEach(() => {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  state = onlineStatus();
  calls = [];
  fetcher = vi.fn(async (path: string, init: RequestInit = {}) => {
    const url = new URL(path, "http://localhost");
    const method = init.method ?? "GET";
    const body = init.body ? JSON.parse(String(init.body)) : null;
    if (path === "/api/online-catalogue") {
      return respond({
        available: true,
        sources: [
          { id: "reko", label: "Reko", defaultBaseUrl: state.defaultBaseUrl },
        ],
      });
    }
    if (path === base) return respond(state);
    if (url.pathname === `${base}/search`) {
      const offset = Number(url.searchParams.get("offset"));
      return respond({
        total: 30,
        offset,
        limit: 25,
        entries: [
          onlineEntry({
            id: offset ? "fictional-2" : "fictional-1",
            name: offset ? "Second result" : "Fictional dimmer",
          }),
          onlineEntry({
            id: "old-1",
            name: "Unsupported VD1",
            fileType: "vd1",
            fileName: "old.vd1",
            importable: false,
          }),
        ],
      });
    }
    if (path === `${base}/sync`) {
      state = {
        ...state,
        index: { ...state.index!, lastCheckedAt: "2026-01-03T00:00:00Z" },
      };
      return respond({ outcome: "notModified", status: state });
    }
    if (path === `${base}/credentials`) {
      state = {
        ...state,
        credentials:
          method === "DELETE"
            ? { state: "empty", mode: null, user: null, problem: null }
            : {
                state: "stored",
                mode: body.mode,
                user: body.user ?? null,
                problem: null,
              },
      };
      return respond(state);
    }
    if (path === `${base}/config`) {
      state = {
        ...state,
        baseUrl: body.baseUrl ?? state.defaultBaseUrl,
        index: null,
        credentials: {
          state: "problem",
          mode: null,
          user: null,
          problem: "vaultOriginMismatch",
        },
      };
      return respond(state);
    }
    if (url.pathname.endsWith("/download") && method === "POST") {
      state = {
        ...state,
        download: {
          state: "done",
          entryId: url.pathname.split("/").at(-2)!,
          receivedBytes: 1,
          totalBytes: 1,
          error: null,
        },
      };
      return respond(state.download);
    }
    if (path === `${base}/download` && method === "DELETE") {
      return respond(state.download);
    }
    throw new Error(`Unexpected ${method} ${path}`);
  });
  vi.stubGlobal("fetch", (path: string, init: RequestInit = {}) => {
    calls.push({
      path,
      method: init.method ?? "GET",
      body: init.body ? JSON.parse(String(init.body)) : null,
    });
    return fetcher(path, init);
  });
});

afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

async function mount(element = <OnlineCatalogue onInstalled={vi.fn()} />) {
  await act(async () => {
    root.render(element);
  });
}

async function click(text: string, scope: ParentNode = host) {
  const button = Array.from(scope.querySelectorAll<HTMLButtonElement>("button")).find(
    (node) => node.textContent === text,
  );
  expect(button, `button ${text}`).toBeTruthy();
  expect(button!.disabled).toBe(false);
  await act(async () => button!.click());
}

async function fill(label: string, value: string, scope: ParentNode = host) {
  const node = Array.from(scope.querySelectorAll("label"))
    .find((node) => node.textContent?.startsWith(label))!
    .querySelector<HTMLInputElement | HTMLSelectElement>("input,select")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      node.tagName === "SELECT"
        ? HTMLSelectElement.prototype
        : HTMLInputElement.prototype,
      "value",
    )!.set!.call(node, value);
    node.dispatchEvent(
      new Event(node.tagName === "SELECT" ? "change" : "input", {
        bubbles: true,
      }),
    );
  });
}

async function submit(scope: ParentNode = host, index = 0) {
  await act(async () =>
    scope
      .querySelectorAll("form")[index]
      .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })),
  );
}

async function choose() {
  await click("Search local index");
  const button = host.querySelector<HTMLButtonElement>(
    ".online-catalogue-results button",
  )!;
  await act(async () => button.click());
}

function modal() {
  return document.body.querySelector<HTMLElement>(
    '[aria-labelledby="online-account-title"]',
  )!;
}

describe("online catalogue explicit action boundaries", () => {
  it("makes no catalogue request for local source or unopened settings, and no sync/search/download on open", async () => {
    await mount(
      <ProductInstallControl
        label="Install"
        className="catalog-install"
        onInstalled={vi.fn()}
      />,
    );
    expect(calls).toEqual([]);
    await fill("Catalogue source", "reko");
    expect(calls.map((call) => call.path)).toEqual([
      "/api/online-catalogue",
      base,
    ]);
    expect(calls.every((call) => call.method === "GET")).toBe(true);
    await mount(<OnlineCatalogueSettings />);
    expect(calls).toHaveLength(2);
    await click("Reko account");
    expect(calls).toHaveLength(3);
    expect(calls.at(-1)?.path).toBe(base);
  });

  it("requires manual sync, records a 304 successful check and preserves the old index on failure", async () => {
    await mount();
    expect(host.textContent).toContain("2026-01-02T00:00:00Z");
    await click("Update index now");
    expect(host.textContent).toContain("The index is unchanged.");
    expect(host.textContent).toContain("2026-01-03T00:00:00Z");
    expect(host.textContent).toContain("2026-01-01T00:00:00Z");
    fetcher.mockResolvedValueOnce(respond({ kind: "serviceUnavailable" }, 503));
    await click("Update index now");
    expect(host.textContent).toContain("2026-01-03T00:00:00Z");
    expect(host.textContent).toContain("temporarily unavailable");
    expect(calls.filter((call) => call.path.includes("/search"))).toHaveLength(0);
  });

  it("searches bounded local pages, exposes application versions and labels VD1 unsupported", async () => {
    await mount();
    await fill("Manufacturer", "FC-4");
    await choose();
    expect(calls.find((call) => call.path.includes("/search"))?.path).toBe(
      `${base}/search?q=FC-4&offset=0&limit=25`,
    );
    expect(host.textContent).toContain(
      "Application 0001 · version 02 · mask 0701",
    );
    expect(host.textContent).toContain("Import not supported (VD1/VD2)");
    expect(
      host.querySelector(".online-catalogue-details")!.textContent,
    ).not.toContain("Prepare import");
    await click("Next page");
    expect(calls.at(-1)?.path).toContain("offset=25&limit=25");
    expect(host.querySelector(".online-catalogue-details")).toBeNull();
  });

  it("resumes only the originally requested download after password save, clears secrets and never imports", async () => {
    await mount();
    await choose();
    await click("Download selected file");
    expect(modal()).toBeTruthy();
    expect(
      calls.some((call) => call.method === "POST" && call.path.includes("/download")),
    ).toBe(false);
    await fill("Username", "fictional-user", modal());
    await fill("Account password", "canary-password", modal());
    await submit(modal());
    expect(calls.find((call) => call.path.endsWith("/credentials"))?.body).toEqual({
      mode: "password",
      user: "fictional-user",
      password: "canary-password",
    });
    expect(calls.filter((call) => call.method === "POST")).toEqual([
      {
        method: "POST",
        path: `${base}/entries/fictional-1/download?sha256=${"a".repeat(64)}`,
        body: null,
      },
    ]);
    expect(modal()).toBeNull();
    expect(document.body.innerHTML).not.toContain("canary-password");
    expect(localStorage.length).toBe(0);
    expect(host.textContent).toContain("Prepare import…");
  });

  it("opens the account dialog for a polled auth failure of the current download, then resumes once after save", async () => {
    vi.useFakeTimers();
    state = onlineStatus({
      credentials: { state: "stored", mode: "token", user: null, problem: null },
    });
    await mount();
    await choose();
    state = {
      ...state,
      download: {
        state: "running",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: 1,
        error: null,
      },
    };
    fetcher.mockResolvedValueOnce(respond(state.download));
    await click("Download selected file");
    expect(modal()).toBeNull();
    state = {
      ...state,
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "tokenRejected" },
      },
    };
    await act(async () => {
      await vi.advanceTimersByTimeAsync(750);
    });
    expect(modal()).toBeTruthy();
    await fill("Username", "fictional-user", modal());
    await fill("Account password", "canary-password", modal());
    await submit(modal());
    const posts = calls.filter((call) => call.method === "POST");
    expect(posts).toEqual([
      {
        method: "POST",
        path: `${base}/entries/fictional-1/download?sha256=${"a".repeat(64)}`,
        body: null,
      },
      {
        method: "POST",
        path: `${base}/entries/fictional-1/download?sha256=${"a".repeat(64)}`,
        body: null,
      },
    ]);
    expect(calls.filter((call) => call.method === "PUT")).toEqual([
      {
        method: "PUT",
        path: `${base}/credentials`,
        body: {
          mode: "password",
          user: "fictional-user",
          password: "canary-password",
        },
      },
    ]);
    expect(modal()).toBeNull();
  });

  it("does not open the account dialog for a retained failed status on mount or after a stale poll", async () => {
    vi.useFakeTimers();
    state = onlineStatus({
      download: {
        state: "failed",
        entryId: "old-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "tokenRejected" },
      },
    });
    await mount();
    expect(modal()).toBeNull();
    await choose();
    state = {
      ...state,
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "tokenRejected" },
      },
    };
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1500);
    });
    expect(modal()).toBeNull();
    expect(calls.some((call) => call.method === "POST")).toBe(false);
  });

  it("does not resume a login action after its source was replaced", async () => {
    await mount();
    await choose();
    await click("Download selected file");
    await fill("Authentication method", "token", modal());
    await fill("Access token", "canary-token", modal());
    let release!: (value: Response) => void;
    fetcher.mockImplementationOnce(
      () =>
        new Promise<Response>((resolve) => {
          release = resolve;
        }),
    );
    await act(async () => {
      modal()
        .querySelector("form")!
        .dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });
    expect(
      modal().querySelector<HTMLInputElement>('input[type="password"]')!.value,
    ).toBe("");
    await mount(<p>Local replacement</p>);
    await act(async () => {
      release(
        respond(
          onlineStatus({
            credentials: {
              state: "stored",
              mode: "token",
              user: null,
              problem: null,
            },
          }),
        ),
      );
    });
    expect(calls.filter((call) => call.path.includes("/entries/")).length).toBe(0);
    expect(host.textContent).toBe("Local replacement");
  });

  it("waits for terminal cancellation, polling only own-backend status and never auto-imports", async () => {
    vi.useFakeTimers();
    state = onlineStatus({
      credentials: { state: "stored", mode: "token", user: null, problem: null },
    });
    await mount();
    await choose();
    state = {
      ...state,
      download: {
        state: "running",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: 1,
        error: null,
      },
    };
    fetcher.mockResolvedValueOnce(respond(state.download));
    await click("Download selected file");
    await click("Stop download");
    expect(host.textContent).toContain("waiting for confirmation");
    state = {
      ...state,
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "cancelled" },
      },
    };
    await act(async () => {
      await vi.advanceTimersByTimeAsync(750);
    });
    expect(host.textContent).toContain("The download was stopped.");
    expect(host.textContent).not.toContain("waiting for confirmation");
    expect(host.querySelectorAll('[role="alert"]')).toHaveLength(1);
    const count = calls.length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1500);
    });
    expect(calls).toHaveLength(count);
  });

  it.each([
    ["selectionChanged", 409],
    ["integrity", 502],
    ["dailyLimit", 429],
    ["rateLimited", 429],
    ["invalidCredentials", 409],
    ["tokenRejected", 409],
    ["forbidden", 403],
    ["notFound", 404],
    ["serviceUnavailable", 503],
  ] as const)(
    "surfaces %s without remote sync, import or automatic retry",
    async (kind, status) => {
      vi.useFakeTimers();
      state = onlineStatus({
        credentials: {
          state: "stored",
          mode: "token",
          user: null,
          problem: null,
        },
      });
      await mount();
      await choose();
      fetcher.mockResolvedValueOnce(
        respond(
          {
            kind,
            message: "provider-secret-must-not-render",
            resetAt: "2026-01-04T00:00:00Z",
            retryAfterSecs: 10,
          },
          status,
        ),
      );
      await click("Download selected file");
      expect(document.body.querySelector('[role="alert"]')).toBeTruthy();
      expect(calls.filter((call) => call.method === "POST")).toEqual([
        {
          method: "POST",
          path: `${base}/entries/fictional-1/download?sha256=${"a".repeat(64)}`,
          body: null,
        },
      ]);
      expect(calls.some((call) => call.path.endsWith("/sync"))).toBe(false);
      expect(
        calls.some((call) => call.path.includes("install-legacy")),
      ).toBe(false);
      const attempted = calls.filter((call) => call.method === "POST").length;
      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });
      expect(document.body.innerHTML).not.toContain(
        "provider-secret-must-not-render",
      );
      expect(calls.filter((call) => call.method === "POST")).toHaveLength(
        attempted,
      );
    },
  );

  it("clears password/token drafts after failure and stores settings secrets only through credentials API", async () => {
    await mount(
      <OnlineCatalogueAccount
        status={state}
        configure
        onSaved={vi.fn()}
        onChanged={vi.fn()}
      />,
    );
    await fill("Authentication method", "token");
    await fill("Access token", "canary-token");
    fetcher.mockResolvedValueOnce(respond({ kind: "vaultKeyMissing" }, 409));
    await submit(host, 1);
    expect(
      host.querySelector<HTMLInputElement>('input[type="password"]')!.value,
    ).toBe("");
    expect(host.textContent).toContain("separate key");
    expect(localStorage.length).toBe(0);
    await fill("Access token", "new-canary");
    await submit(host, 1);
    expect(
      calls
        .filter((call) => call.path.endsWith("/credentials"))
        .map((call) => call.body),
    ).toEqual([
      { mode: "token", token: "canary-token" },
      { mode: "token", token: "new-canary" },
    ]);
    expect(calls.some((call) => call.path === "/api/settings")).toBe(false);
    await fill("HTTPS provider", "https://different.example/");
    await submit(host, 0);
    expect(calls.find((call) => call.path.endsWith("/config"))?.body).toEqual({
      baseUrl: "https://different.example/",
    });
  });

  it("hands the cached modern file through the real controller and feed client only after profile consent", async () => {
    state = onlineStatus({
      credentials: { state: "stored", mode: "token", user: null, problem: null },
    });
    const standIn = fakeApi();
    const receiving = {
      ...jobStatus(),
      source: { ...jobStatus().source, name: "fictional.knxprod" },
      profile: {
        ...jobStatus().profile,
        name: "standard" as const,
        maxExpandedBytes: "4294967296" as const,
      },
    };
    const ready = {
      ...jobStatus("ready", "2"),
      source: { ...jobStatus("ready", "2").source, name: "fictional.knxprod" },
      profile: receiving.profile,
    };
    const jobsApi: CatalogueJobsApi = {
      ...standIn,
      create: vi.fn(async () => receiving),
      status: vi.fn(async () => ready),
    };
    const controller = new CatalogueJobsController(jobsApi);
    const start = vi.spyOn(controller, "startSource");
    await mount(
      <CatalogueJobsProvider controller={controller}>
        <OnlineCatalogue onInstalled={vi.fn()} />
      </CatalogueJobsProvider>,
    );
    await choose();
    await click("Download selected file");
    expect(start).not.toHaveBeenCalled();
    await click("Prepare import…");
    expect(start).not.toHaveBeenCalled();
    fetcher.mockResolvedValueOnce(respond(ready, 202));
    await click(en["catalogJob.start"]);
    expect(controller.getSnapshot().status?.phase).toBe("ready");
    expect(controller.getSnapshot().error).toBeNull();
    expect(start.mock.calls[0].slice(0, 4)).toEqual([
      { name: "fictional.knxprod", size: 1 },
      "standard",
      false,
      null,
    ]);
    expect(jobsApi.upload).not.toHaveBeenCalled();
    expect(calls.filter((call) => call.path.includes("/feed/"))).toEqual([
      {
        method: "POST",
        path: `${base}/jobs/${"b".repeat(32)}/feed/fictional-1?sha256=${"a".repeat(64)}`,
        body: null,
      },
    ]);
    expect(jobsApi.apply).not.toHaveBeenCalled();
  });

  it("keeps Stop pending after a polling transport failure until explicit terminal readback", async () => {
    vi.useFakeTimers();
    state = onlineStatus({
      download: {
        state: "running",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: 1,
        error: null,
      },
    });
    await mount();
    await click("Stop download");
    fetcher.mockRejectedValueOnce(new Error("lost poll"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(750);
    });
    expect(host.textContent).toContain("waiting for confirmation");
    state = {
      ...state,
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "cancelled" },
      },
    };
    await click("Check local status");
    expect(host.textContent).not.toContain("waiting for confirmation");
    expect(host.textContent).toContain("The download was stopped.");
    expect(host.querySelectorAll('[role="alert"]')).toHaveLength(1);
  });

  it("shows one quota notice when retained download failure and source limit describe the same refusal", async () => {
    state = onlineStatus({
      limit: {
        kind: "dailyLimit",
        resetAt: "2026-01-04T00:00:00Z",
        retryAfterSecs: 10,
      },
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: {
          kind: "dailyLimit",
          resetAt: "2026-01-04T00:00:00Z",
          retryAfterSecs: 10,
        },
      },
    });
    await mount();
    expect(host.querySelectorAll('[role="alert"]')).toHaveLength(1);
    expect(host.querySelector('[role="alert"]')!.textContent).toContain(
      "2026-01-04T00:00:00Z",
    );
  });

  it("notifies the catalogue after a committed legacy import even when its source view was replaced", async () => {
    const installed = vi.fn();
    await mount(<OnlineCatalogue onInstalled={installed} />);
    fetcher.mockResolvedValueOnce(
      respond({
        total: 1,
        offset: 0,
        limit: 25,
        entries: [
          onlineEntry({
            fileType: "vd4",
            fileName: "fictional.vd4",
            downloaded: true,
          }),
        ],
      }),
    );
    await choose();
    let release!: (response: Response) => void;
    fetcher.mockImplementationOnce(
      () =>
        new Promise<Response>((resolve) => {
          release = resolve;
        }),
    );
    await click("Prepare import…");
    await mount(<p>Local replacement</p>);
    const report = {
      payloadSha256: "a".repeat(64),
      originalSha256: "b".repeat(64),
      namespace: "LX1A2B3C4D",
      skipped: false,
      programs: ["M-1092_A-LX1A2B3C4D-300"],
      catalogItems: 1,
      parameters: 0,
      parameterRefs: 0,
      comObjectRefs: 0,
      translations: 0,
      diagnostics: [],
      password: "none",
      remembered: false,
      rememberProblem: null,
    } satisfies import("./api").LegacyInstallReport;
    await act(async () => release(respond(report)));
    expect(installed).toHaveBeenCalledTimes(1);
    expect(host.textContent).toBe("Local replacement");
  });

  it("renders a retained terminal refusal when reopening the source", async () => {
    state = onlineStatus({
      download: {
        state: "failed",
        entryId: "fictional-1",
        receivedBytes: 0,
        totalBytes: null,
        error: { kind: "integrity" },
      },
    });
    await mount();
    const alerts = host.querySelectorAll('[role="alert"]');
    expect(alerts).toHaveLength(1);
    expect(alerts[0].textContent).toContain(en["online.error.integrity"]);
  });
});
