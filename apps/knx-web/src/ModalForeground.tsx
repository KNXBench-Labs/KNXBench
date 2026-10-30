/** Scopes retained descendant modals to their visible foreground owner. */
import { createContext, useContext } from "react";
import type { ReactNode } from "react";

const Foreground = createContext(true);

export function ModalForeground({ active, children }: { active: boolean; children: ReactNode }) {
  const parentActive = useContext(Foreground);
  return <Foreground.Provider value={parentActive && active}>{children}</Foreground.Provider>;
}

export function useModalForeground(): boolean {
  return useContext(Foreground);
}
