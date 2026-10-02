/** Isolates modal backgrounds with document-local DOM leases. */
// SPDX-License-Identifier: AGPL-3.0-or-later
interface ModalState {
  panels: HTMLElement[];
  saved: Map<Element, [string | null, string | null]>;
  observer: MutationObserver | null;
  previousFocus: HTMLElement | null;
}

// A DOM resource lease, not application/domain state. Weak keys isolate windows
// and release the record when the final modal closes.
const documents = new WeakMap<Document, ModalState>();

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

export function isolateModalBackground(panel: HTMLElement, focus: () => void): () => boolean {
  const document = panel.ownerDocument;
  let state = documents.get(document);
  if (!state) {
    state = { panels: [], saved: new Map(), observer: null, previousFocus: document.activeElement instanceof HTMLElement ? document.activeElement : null };
    documents.set(document, state);
  }
  const owned = state;
  restore(owned);
  owned.panels.push(panel);
  // Release the earlier dialog's inert ancestry before focusing the new one;
  // hide background only after focus has left it.
  focus();
  excludeBackground(panel, owned);
  if (!owned.observer) {
    owned.observer = new MutationObserver(() => {
      if (documents.get(document) !== owned) return;
      restore(owned);
      owned.panels = owned.panels.filter((item) => item.isConnected);
      const top = owned.panels.at(-1);
      if (top) excludeBackground(top, owned);
      else {
        owned.observer?.disconnect();
        documents.delete(document);
      }
    });
    owned.observer.observe(document.body, { childList: true, subtree: true });
  }
  return () => {
    const wasTop = owned.panels.at(-1) === panel;
    restore(owned);
    owned.panels = owned.panels.filter((item) => item !== panel && item.isConnected);
    const top = owned.panels.at(-1);
    if (top) excludeBackground(top, owned);
    else {
      owned.observer?.disconnect();
      documents.delete(document);
      if (owned.previousFocus?.isConnected && !owned.previousFocus.closest("[inert]")) owned.previousFocus.focus();
    }
    return wasTop;
  };
}
