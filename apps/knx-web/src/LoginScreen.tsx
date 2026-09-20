/** The password screen a server deployment shows before it will let the workbench be used. */
//! ADR-0026's frontend half, the part a human sees. One field, one button,
//! one error line — and, because this is the first thing anyone meets on a
//! server deployment, it is drawn entirely from the theme tokens the rest
//! of the application uses, so it looks deliberate in all five palettes and
//! moves in whichever of the two motion styles is selected.
//!
//! Nothing here stores, logs, or puts the password anywhere but the body of
//! the one POST that verifies it.
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { useTranslate } from "./i18n";

/**
 * Why the screen is up. It changes exactly one thing — the sentence above
 * the field — but that sentence is the difference between "this server
 * wants a password" and "your session ended, and here is what became of
 * your work".
 */
export type LoginReason =
  /** First load: the application has never been rendered in this tab. */
  | "initial"
  /** A 401 arrived mid-session: the workbench is still mounted behind this. */
  | "expired"
  /** The user pressed Log out. Same mounted workbench, no bad news to break. */
  | "signedOut";

const NOTICE_KEY = {
  initial: null,
  expired: "login.expiredNotice",
  signedOut: "login.signedOutNotice",
} as const;

export default function LoginScreen(props: {
  reason: LoginReason;
  onAuthenticated: () => void;
}) {
  const { reason, onAuthenticated } = props;
  const t = useTranslate();
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Same belt-and-braces as `NewProjectDialog`: `pending` is state, and two
  // submits in the same tick would both read the stale `false`. The server
  // serialises attempts behind a one-permit semaphore and delays every
  // failure on purpose, so a queued second guess is not a wasted round trip
  // but several wasted seconds.
  const inFlightRef = useRef(false);
  const fieldRef = useRef<HTMLInputElement>(null);

  // Autofocus by effect rather than the `autoFocus` attribute: the attribute
  // only acts on the initial mount of the whole tree, and this screen can
  // also appear long afterwards, over a workbench that is already running.
  useEffect(() => {
    fieldRef.current?.focus();
  }, []);

  async function submit() {
    if (inFlightRef.current || pending || password === "") return;
    inFlightRef.current = true;
    setPending(true);
    setError(null);
    try {
      await api.login(password);
      // Cleared before the handover, so the secret does not sit in this
      // component's state for however long the unmount takes.
      setPassword("");
      onAuthenticated();
    } catch (e) {
      // 401 is the one answer worth translating: it is expected, it means
      // exactly one thing, and the server's own English "invalid password"
      // would otherwise be the single untranslated string on the screen.
      // Anything else — 400 "authentication is not enabled on this server",
      // a proxy's 502, a dead network — is reported in the server's words,
      // because inventing a friendlier sentence would be inventing a
      // diagnosis.
      setError(api.errorStatus(e) === 401 ? t("login.rejected") : api.errorMessage(e));
      // Back to the field with the wrong answer selected: one keystroke
      // replaces it, which is what a typo wants, and no separate "clear"
      // step leaves a stale secret in the DOM either.
      fieldRef.current?.focus();
      fieldRef.current?.select();
    } finally {
      inFlightRef.current = false;
      setPending(false);
    }
  }

  const noticeKey = NOTICE_KEY[reason];

  return (
    <div className="login-screen" data-reason={reason}>
      <div
        className="login-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="login-title"
        // The notice is present in the dialog's first paint, and a live
        // region announces *mutations*, so `role="status"` alone would let
        // "your session ended, and the server may have forgotten your
        // project" pass a screen-reader user by entirely. Naming it as the
        // dialog's description has it read out when focus lands inside.
        aria-describedby={noticeKey ? "login-notice" : undefined}
      >
        <p className="eyebrow">{t("login.eyebrow")}</p>
        <h1 className="login-title" id="login-title">
          <span className="brand-mark" aria-hidden="true">
            K
          </span>
          <span className="login-wordmark">KNXBench</span>
        </h1>
        <p className="login-intro">{t("login.intro")}</p>
        {/* `role="status"` rather than `alert`: this explains the situation
            the user arrived in, it does not report a failure of anything
            they just did. The error below is the `alert`. */}
        {noticeKey && (
          <p className="login-notice" id="login-notice" role="status">
            {t(noticeKey)}
          </p>
        )}
        <form
          className="login-form"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <label className="login-field" htmlFor="login-password">
            <span className="login-field-label">{t("login.password")}</span>
            <input
              id="login-password"
              ref={fieldRef}
              type="password"
              name="password"
              // The whole point of the right token here: a password manager
              // offers to fill this field instead of guessing at it.
              autoComplete="current-password"
              value={password}
              aria-invalid={error !== null}
              aria-describedby={error === null ? undefined : "login-error"}
              onChange={(e) => setPassword(e.target.value)}
            />
          </label>
          {error !== null && (
            <p className="field-error login-error" id="login-error" role="alert">
              {error}
            </p>
          )}
          <button
            type="submit"
            className="primary-action login-submit"
            // Disabled while the attempt is out, because the server's
            // failure delay is real time the user would otherwise queue up
            // several times over, and on an empty field, because an empty
            // guess earns that delay for nothing.
            disabled={pending || password === ""}
          >
            {pending ? t("login.pending") : t("login.submit")}
          </button>
        </form>
        <p className="login-footnote">{t("login.footnote")}</p>
      </div>
    </div>
  );
}
