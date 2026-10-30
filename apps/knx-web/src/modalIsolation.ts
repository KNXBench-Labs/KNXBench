/** Isolates modal backgrounds with document-local, suspendable DOM leases. */
// SPDX-License-Identifier: AGPL-3.0-or-later
interface ModalLease {
  panel: HTMLElement;
  focus: () => void;
  active: boolean;
}
interface ModalState {
  panels: ModalLease[];
  top: ModalLease | undefined;
  saved: Map<Element, [string | null, string | null]>;
  observer: MutationObserver | null;
  previousFocus: HTMLElement | null;
}

// A DOM resource lease, not application/domain state. Weak keys isolate windows
// and release the record when the final modal closes. Paused leases keep stack order.
const documents = new WeakMap<Document, ModalState>();

export function canRestoreModalFocus(target: HTMLElement | null | undefined): target is HTMLElement {
  return !!target?.isConnected && !target.closest("[hidden], [inert], [aria-hidden='true']");
}

function restore(state: ModalState) {
  for (const [element, values] of state.saved) {
    for (const [index, attribute] of ["inert", "aria-hidden"].entries()) {
      const value = values[index];
      if (value === null) element.removeAttribute(attribute);
      else element.setAttribute(attribute, value);
    }
  }
  state.saved.clear();
}

function excludeBackground(panel: HTMLElement, state: ModalState) {
  let branch: Element = panel.parentElement ?? panel;
  while (branch.parentElement) {
    for (const sibling of branch.parentElement.children) {
      if (sibling === branch) continue;
      state.saved.set(sibling, [sibling.getAttribute("inert"), sibling.getAttribute("aria-hidden")]);
      sibling.setAttribute("inert", "");
      sibling.setAttribute("aria-hidden", "true");
    }
    branch = branch.parentElement;
    if (branch === panel.ownerDocument.body) break;
  }
}

function refresh(document: Document, state: ModalState) {
  restore(state);
  const previous = state.top;
  // A commit can hide a retained branch before its effect suspends the lease.
  // Never focus or isolate a hidden panel during that handoff.
  const top = [...state.panels].reverse().find(item => item.active && item.panel.isConnected && !item.panel.closest("[hidden]"));
  state.top = top;
  if (top) {
    if (previous !== top) top.focus();
    excludeBackground(top.panel, state);
    if (!state.observer) {
      state.observer = new MutationObserver(() => {
        if (documents.get(document) === state) refresh(document, state);
      });
      state.observer.observe(document.body, { childList: true, subtree: true });
    }
  } else {
    state.observer?.disconnect();
    state.observer = null;
    if (previous && canRestoreModalFocus(state.previousFocus)) state.previousFocus.focus();
  }
}

export function isolateModalBackground(panel: HTMLElement, focus: () => void, active = true) {
  const document = panel.ownerDocument;
  let state = documents.get(document);
  if (!state) {
    state = { panels: [], top: undefined, saved: new Map(), observer: null, previousFocus: document.activeElement instanceof HTMLElement ? document.activeElement : null };
    documents.set(document, state);
  }
  const owned = state;
  const lease: ModalLease = { panel, focus, active };
  owned.panels.push(lease);
  refresh(document, owned);
  return {
    setActive(value: boolean) {
      if (lease.active === value) return;
      lease.active = value;
      refresh(document, owned);
    },
    release() {
      const wasTop = owned.top === lease;
      owned.panels = owned.panels.filter(item => item !== lease);
      refresh(document, owned);
      if (owned.panels.length === 0) documents.delete(document);
      return wasTop;
    },
  };
}
