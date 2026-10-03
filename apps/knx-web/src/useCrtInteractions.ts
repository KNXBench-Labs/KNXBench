/** Mount one disposable feedback controller for the real workspace shell. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect, type RefObject } from "react";
import { installCrtInteractions } from "./crtInteractions";

export function useCrtInteractions(scope: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    if (scope.current) return installCrtInteractions(scope.current);
  }, [scope]);
}
