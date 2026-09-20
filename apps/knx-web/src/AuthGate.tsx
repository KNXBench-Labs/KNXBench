/** Asks the server whether a password is wanted, then renders the app or the login screen. */
//! The other half of ADR-0026. `GET /api/auth/status` is answered before
//! anything else is rendered, because the alternative is what the frontend
//! did until now: mount the whole workbench, fire a dozen requests, and
//! collect a dozen 401s.
//!
//! Three outcomes, exactly as the endpoint describes them:
//!
//! - `required: false` — render the application and nothing else. This is
//!   the branch that keeps a login screen off the desktop shell.
//! - `required: true, authenticated: true` — render the application.
//! - `required: true, authenticated: false` — render the login screen.
//!
//! It takes its children as a function rather than as elements so that
//! `main.tsx` keeps deciding which of the two roles (workbench or
//! diagnostics companion) it is mounting; this file imports neither, and so
//! does not drag the editor's module graph into the companion window.
import { useCallback, useEffect, useState, type ReactNode } from "react";
import * as api from "./api";
import { useTranslate } from "./i18n";
import LoginScreen, { type LoginReason } from "./LoginScreen";
import { subscribeSessionExpired, type SessionControls } from "./session";

export default function AuthGate(props: {
  children: (session: SessionControls) => ReactNode;
}) {
  const { children } = props;
  const t = useTranslate();
  // `false` until `GET /api/auth/status` has answered. Nothing is rendered
  // in the meantime — least of all a login screen, which would flash on
  // every load of a server that does not want one.
  const [ready, setReady] = useState(false);
  const [required, setRequired] = useState(false);
  const [lockedBy, setLockedBy] = useState<LoginReason | null>(null);
  // Whether the application has been mounted at least once in this tab.
  //
  // This is the whole of "preserve unsaved work across a mid-session 401":
  // once it is true it never goes false again, so the application stays
  // mounted underneath the login screen — with its project tree, its
  // selection, its open panels and any half-typed dialog exactly as they
  // were — and reappears untouched when the password is accepted. Unmounting
  // it would throw all of that away, and the server offers no endpoint that
  // hands the current project back, so nothing could put it on screen again
  // short of the user reopening the file.
  const [everOpen, setEverOpen] = useState(false);

  const open = useCallback((authRequired: boolean) => {
    setRequired(authRequired);
    setEverOpen(true);
    setLockedBy(null);
    setReady(true);
  }, []);

  useEffect(() => {
    let cancelled = false;
    api
      .authStatus()
      .then((status) => {
        if (cancelled) return;
        if (status.required && !status.authenticated) {
          setRequired(true);
          setLockedBy("initial");
          setReady(true);
          return;
        }
        open(status.required);
      })
      .catch(() => {
        // The status endpoint is unauthenticated and answers 200 on a server
        // that has authentication switched off, so a rejection here is not
        // "you are logged out" — it is "this server did not answer at all".
        // Rendering a login screen on the strength of that would be a
        // guess, and a wrong one on the desktop shell. So the gate fails
        // open into exactly the behaviour this frontend had before it
        // existed; the 401 subscription below is still armed, and the first
        // refused request puts the login screen up with the server's own
        // word for it rather than this file's.
        if (cancelled) return;
        open(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(
    () =>
      subscribeSessionExpired(() => {
        // A session that idled out, or a server that was restarted and
        // forgot every token it had issued. `required` is set rather than
        // read: the server has just demonstrated that it wants one, whatever
        // the status call earlier believed.
        setRequired(true);
        setLockedBy((previous) => previous ?? "expired");
      }),
    [],
  );

  const logout = useCallback(() => {
    // Locked either way. `POST /api/auth/logout` is idempotent and answers
    // 200 even with no session, so the only way past the `catch` is a
    // network that is down — in which case the cookie may well outlive this
    // click, but the user asked for the screen to be shut and leaving their
    // project on display instead would be the worse of the two wrong
    // answers.
    void api.logout().catch(() => undefined).finally(() => setLockedBy("signedOut"));
  }, []);

  if (!ready) {
    return (
      <p className="auth-checking" role="status">
        {t("login.checking")}
      </p>
    );
  }

  const session: SessionControls = { required, logout };

  return (
    <>
      {everOpen && (
        // `display: contents`, so the application's own root element keeps
        // being the flex/grid child it was; `inert` while locked, so the
        // workbench sitting behind an opaque login screen cannot be reached
        // by Tab, by a click, or by a screen reader's virtual cursor.
        <div className="auth-gate-app" inert={lockedBy !== null}>
          {children(session)}
        </div>
      )}
      {lockedBy !== null && (
        <LoginScreen reason={lockedBy} onAuthenticated={() => open(true)} />
      )}
    </>
  );
}
