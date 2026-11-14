/** Returns settled preview and explicit Cancel focus to their exact, still-current initiator. */
import { useCallback, useLayoutEffect, useRef } from "react";

type Request = {
  trigger: HTMLButtonElement;
  abandoned: boolean;
  restoreFrom: HTMLElement | null;
  stopWatching: () => void;
};

/** Local to one editor identity and client; passive revocation never requests focus. */
export function usePreviewFocus(owner: string, client: object) {
  const current = useRef<Request | null>(null);
  const revoke = useCallback(() => {
    current.current?.stopWatching();
    current.current = null;
  }, []);
  // Retire before the restoration effect, including on a same-project revision rebase.
  useLayoutEffect(() => revoke, [owner, client, revoke]);
  useLayoutEffect(() => {
    const request = current.current;
    if (!request?.restoreFrom) return;
    const from = request.restoreFrom;
    request.restoreFrom = null;
    request.stopWatching();
    const { trigger } = request;
    const document = trigger.ownerDocument;
    const active = document.activeElement;
    if (!request.abandoned && trigger.isConnected && !trigger.disabled &&
        !trigger.closest('[hidden], [inert], [aria-hidden="true"]') &&
        (active === from || active === document.body || active === null)) {
      trigger.focus();
    }
  });
  function begin(trigger: HTMLButtonElement) {
    revoke();
    const document = trigger.ownerDocument;
    const request: Request = {
      trigger, abandoned: document.activeElement !== trigger,
      restoreFrom: null, stopWatching: () => undefined,
    };
    // Native disabled may drop focus to the document; that alone is not user navigation.
    const focus = (event: FocusEvent) => {
      if (event.target !== trigger && event.target !== document.body) request.abandoned = true;
    };
    const pointer = (event: PointerEvent) => {
      if (!trigger.contains(event.target as Node)) request.abandoned = true;
    };
    const key = (event: KeyboardEvent) => { if (event.key === "Tab") request.abandoned = true; };
    const blur = () => { request.abandoned = true; };
    document.addEventListener("focusin", focus, true);
    document.addEventListener("pointerdown", pointer, true);
    document.addEventListener("keydown", key, true);
    document.defaultView?.addEventListener("blur", blur);
    request.stopWatching = () => {
      document.removeEventListener("focusin", focus, true);
      document.removeEventListener("pointerdown", pointer, true);
      document.removeEventListener("keydown", key, true);
      document.defaultView?.removeEventListener("blur", blur);
    };
    current.current = request;
    return request;
  }
  function settle(request: Request) {
    if (current.current === request) request.restoreFrom = request.trigger;
    // The caller's busy=false commit re-enables the button before the layout effect.
  }
  function cancel(button: HTMLButtonElement) {
    const request = current.current;
    if (!request) return;
    request.stopWatching();
    const active = button.ownerDocument.activeElement;
    request.abandoned = active !== button && active !== button.ownerDocument.body && active !== null;
    request.restoreFrom = button;
    // The caller removes this Cancel in the same commit; no timers or fallback target.
  }
  return { begin, settle, cancel, revoke };
}
