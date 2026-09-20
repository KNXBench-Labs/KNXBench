/** Tests the auth-status branches, login, logout, and the bounce back on a mid-session 401. */
// @vitest-environment happy-dom
//
// T01b / ADR-0026. The server half of authentication is tested in
// `apps/knx-server`; what is pinned here is the decision this gate makes on
// the strength of the server's answer, and — the part no server test can
// see — that a session ending mid-flight does not take the user's unsaved
// work with it.
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  authStatus: vi.fn(),
  login: vi.fn(),
  logout: vi.fn().mockResolvedValue({ authenticated: false }),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (e: unknown) => (e instanceof Error ? e.message : String(e)),
  errorStatus: (e: unknown) =>
    e instanceof Error && typeof (e as Error & { status?: number }).status === "number"
      ? (e as Error & { status: number }).status
      : undefined,
}));

import AuthGate from "./AuthGate";
import { messages as en } from "./messages/en";
import { notifySessionExpired, resetSessionListenersForTests, type SessionControls } from "./session";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  apiMock.logout.mockResolvedValue({ authenticated: false });
  resetSessionListenersForTests();
  window.localStorage.removeItem(UI_LANGUAGE_STORAGE_KEY);
  resetUiLanguageForTests();
});

/**
 * A stand-in for the application: it counts its own mounts and keeps a
 * scrap of state across re-renders, which is the only way a test can tell
 * "still mounted behind the login screen" apart from "unmounted and built
 * again from nothing". The real `App` is not used here on purpose — this
 * file is about the gate's decisions, and `App.test.tsx` covers the one
 * thing the gate hands it (the logout control).
 */
let mountCount = 0;

function StubApp(props: { session: SessionControls }) {
  // A lazy `useState` initialiser runs exactly once per mount, which makes
  // it the cheapest "was this component built again?" probe there is.
  const [serial] = useState(() => (mountCount += 1));
  return (
    <main className="stub-app">
      <p className="stub-app-mounts">{serial}</p>
      <button className="stub-app-logout" onClick={props.session.logout}>
        {props.session.required ? "log out" : "no session"}
      </button>
    </main>
  );
}

async function mount() {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<AuthGate>{(session) => <StubApp session={session} />}</AuthGate>);
  });
  return host;
}

function loginPanel(): HTMLElement | null {
  return host!.querySelector<HTMLElement>(".login-panel");
}

function passwordField(): HTMLInputElement {
  return host!.querySelector<HTMLInputElement>("#login-password")!;
}

function submitButton(): HTMLButtonElement {
  return host!.querySelector<HTMLButtonElement>(".login-submit")!;
}

// Through the native setter, not `field.value = x`: React's own change
// tracking would otherwise treat the assignment as a no-op and never call
// `onChange` — the same reasoning `CommandPalette.test.tsx` documents.
async function typePassword(value: string) {
  const field = passwordField();
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => {
    setter.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("the auth status gate", () => {
  it("renders the application and no login screen when no password is required", async () => {
    apiMock.authStatus.mockResolvedValue({ required: false, authenticated: true });
    await mount();

    expect(host!.querySelector(".stub-app")).not.toBeNull();
    expect(loginPanel()).toBeNull();
    // The desktop shell's whole guarantee: `required: false` means the
    // session controls say there is nothing to log out of.
    expect(host!.querySelector(".stub-app-logout")?.textContent).toBe("no session");
    expect(apiMock.login).not.toHaveBeenCalled();
  });

  it("renders the application when a password is required and already given", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: true });
    await mount();

    expect(host!.querySelector(".stub-app")).not.toBeNull();
    expect(loginPanel()).toBeNull();
    expect(host!.querySelector(".stub-app-logout")?.textContent).toBe("log out");
  });

  it("renders the login screen, and not the application, when a password is required", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    await mount();

    expect(loginPanel()).not.toBeNull();
    expect(host!.querySelector(".stub-app")).toBeNull();
    // First load, so there is nothing to explain away and no notice.
    expect(host!.querySelector(".login-notice")).toBeNull();
    expect(passwordField().getAttribute("autocomplete")).toBe("current-password");
    expect(passwordField().getAttribute("type")).toBe("password");
    expect(document.activeElement).toBe(passwordField());
  });

  it("falls open to the application when the status call itself fails", async () => {
    // A server that did not answer is not a server that refused. Guessing
    // "logged out" here would put a password prompt on a desktop shell
    // that has no password.
    apiMock.authStatus.mockRejectedValue(new Error("connection refused"));
    await mount();

    expect(host!.querySelector(".stub-app")).not.toBeNull();
    expect(loginPanel()).toBeNull();
  });
});

describe("signing in", () => {
  it("renders the application once the password is accepted", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    apiMock.login.mockResolvedValue({ authenticated: true });
    await mount();

    await typePassword("correct horse");
    await act(async () => submitButton().click());

    expect(apiMock.login).toHaveBeenCalledWith("correct horse");
    expect(loginPanel()).toBeNull();
    expect(host!.querySelector(".stub-app")).not.toBeNull();
    expect(host!.querySelector(".stub-app-logout")?.textContent).toBe("log out");
  });

  it("shows the error and stays put when the password is refused", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    const refusal = Object.assign(new Error("invalid password"), { status: 401 });
    apiMock.login.mockRejectedValue(refusal);
    await mount();

    await typePassword("hunter2");
    await act(async () => submitButton().click());

    const error = host!.querySelector<HTMLElement>(".login-error");
    expect(error?.textContent).toBe(en["login.rejected"]);
    // `role="alert"` is how the message reaches a screen reader at all —
    // the field is still focused, so nothing else would read it out.
    expect(error?.getAttribute("role")).toBe("alert");
    expect(passwordField().getAttribute("aria-describedby")).toBe("login-error");
    expect(loginPanel()).not.toBeNull();
    expect(host!.querySelector(".stub-app")).toBeNull();
    // The server's own English is not what a German reader gets.
    expect(error?.textContent).not.toBe("invalid password");
  });

  it("reports anything that is not a refusal in the server's own words", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    apiMock.login.mockRejectedValue(
      Object.assign(new Error("authentication is not enabled on this server"), { status: 400 }),
    );
    await mount();

    await typePassword("anything");
    await act(async () => submitButton().click());

    expect(host!.querySelector(".login-error")?.textContent).toBe(
      "authentication is not enabled on this server",
    );
  });

  it("disables submit while an attempt is in flight", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    let release: (value: { authenticated: boolean }) => void = () => undefined;
    apiMock.login.mockReturnValue(
      new Promise<{ authenticated: boolean }>((resolve) => {
        release = resolve;
      }),
    );
    await mount();

    // Empty to begin with: an empty guess would earn the server's full
    // penalty delay for nothing.
    expect(submitButton().disabled).toBe(true);
    await typePassword("slow one");
    expect(submitButton().disabled).toBe(false);

    await act(async () => submitButton().click());
    expect(submitButton().disabled).toBe(true);
    expect(submitButton().textContent).toBe(en["login.pending"]);

    // A second click while the first is out must not reach the server: the
    // server serialises attempts behind one permit and delays every
    // failure, so a queued guess is seconds of pointless waiting.
    await act(async () => submitButton().click());
    expect(apiMock.login).toHaveBeenCalledTimes(1);

    await act(async () => {
      release({ authenticated: true });
    });
    expect(loginPanel()).toBeNull();
  });

  it("submits on Enter in the password field", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    apiMock.login.mockResolvedValue({ authenticated: true });
    await mount();

    await typePassword("typed then Enter");
    // What a browser does when Enter is pressed in a single-field form: it
    // asks the form to submit, which is why the button is `type="submit"`
    // inside a real `<form>` rather than a click handler on a `<div>`.
    await act(async () => host!.querySelector<HTMLFormElement>(".login-form")!.requestSubmit());

    expect(apiMock.login.mock.calls).toEqual([["typed then Enter"]]);
    expect(loginPanel()).toBeNull();
  });

  it("names the notice as the dialog's description so it is actually read", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: true });
    await mount();
    await act(async () => notifySessionExpired());

    // A live region announces mutations, and this one is present in the
    // dialog's first paint; without `aria-describedby` the warning that the
    // server may have forgotten the project reaches nobody.
    const panel = host!.querySelector(".login-panel")!;
    const describedBy = panel.getAttribute("aria-describedby");
    expect(describedBy).toBe("login-notice");
    expect(host!.querySelector(`#${describedBy}`)?.textContent).toBe(en["login.expiredNotice"]);
  });

  it("describes nothing when there is no notice to describe", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    await mount();

    expect(host!.querySelector(".login-panel")!.hasAttribute("aria-describedby")).toBe(false);
  });

  it("never puts the password in a URL", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    apiMock.login.mockResolvedValue({ authenticated: true });
    await mount();

    await typePassword("s3cret");
    await act(async () => submitButton().click());

    // The only thing this module ever does with it is hand it to
    // `api.login`, which puts it in a POST body (`api.test.ts` pins that).
    expect(apiMock.login.mock.calls).toEqual([["s3cret"]]);
    expect(window.location.href).not.toContain("s3cret");
    expect(document.body.innerHTML).not.toContain("s3cret");
  });
});

describe("a session that ends mid-flight", () => {
  it("returns to the login screen on a 401 and keeps the application mounted", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: true });
    await mount();
    const mountsWhenOpened = host!.querySelector(".stub-app-mounts")!.textContent;

    await act(async () => notifySessionExpired());

    expect(loginPanel()).not.toBeNull();
    expect(host!.querySelector(".login-notice")?.textContent).toBe(en["login.expiredNotice"]);
    // The point of the exercise: the application is still there, inert
    // behind an opaque screen rather than thrown away and rebuilt. Its
    // project tree, selection and half-typed dialogs survive with it.
    expect(host!.querySelector(".stub-app")).not.toBeNull();
    expect(host!.querySelector(".auth-gate-app")?.hasAttribute("inert")).toBe(true);
    expect(host!.querySelector(".stub-app-mounts")!.textContent).toBe(mountsWhenOpened);

    apiMock.login.mockResolvedValue({ authenticated: true });
    await typePassword("back again");
    await act(async () => submitButton().click());

    expect(loginPanel()).toBeNull();
    expect(host!.querySelector(".auth-gate-app")?.hasAttribute("inert")).toBe(false);
    expect(host!.querySelector(".stub-app-mounts")!.textContent).toBe(mountsWhenOpened);
  });

  it("locks even when the status call had reported no password at all", async () => {
    // The fail-open path above, followed by a server that turns out to
    // want a session after all. The 401 is the server's own word and beats
    // this file's earlier guess.
    apiMock.authStatus.mockRejectedValue(new Error("connection refused"));
    await mount();
    expect(loginPanel()).toBeNull();

    await act(async () => notifySessionExpired());

    expect(loginPanel()).not.toBeNull();
    // And it says "sign in", not "your session ended". The status call
    // never answered, so this tab has never held a session; claiming one
    // expired would be inventing a history for the user's first visit.
    expect(host!.querySelector(".login-notice")).toBeNull();
  });

  it("does not stack a second reason on a screen that is already up", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: false });
    await mount();

    await act(async () => notifySessionExpired());

    // Still the first-load screen: a stray 401 from a request that was
    // already in flight must not rewrite "sign in" into "your session
    // ended".
    expect(host!.querySelector(".login-notice")).toBeNull();
  });
});

describe("logging out", () => {
  it("ends the session, locks the screen, and comes back to the same workbench", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: true });
    await mount();
    const mountsWhenOpened = host!.querySelector(".stub-app-mounts")!.textContent;

    await act(async () => host!.querySelector<HTMLButtonElement>(".stub-app-logout")!.click());

    expect(apiMock.logout).toHaveBeenCalledTimes(1);
    expect(loginPanel()).not.toBeNull();
    expect(host!.querySelector(".login-notice")?.textContent).toBe(en["login.signedOutNotice"]);
    expect(host!.querySelector(".auth-gate-app")?.hasAttribute("inert")).toBe(true);
    expect(host!.querySelector(".stub-app-mounts")!.textContent).toBe(mountsWhenOpened);
  });

  it("locks the screen even if the logout request never arrives", async () => {
    apiMock.authStatus.mockResolvedValue({ required: true, authenticated: true });
    apiMock.logout.mockRejectedValue(new Error("connection refused"));
    await mount();

    await act(async () => host!.querySelector<HTMLButtonElement>(".stub-app-logout")!.click());

    // The cookie may well outlive the click. Leaving the project on
    // display instead would be the worse of the two wrong answers.
    expect(loginPanel()).not.toBeNull();
  });
});
