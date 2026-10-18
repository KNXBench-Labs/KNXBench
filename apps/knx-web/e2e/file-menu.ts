/** Browser-test helper: opens one of the File menu submenus (UI-03) once the menu is open. */
import { expect, type Page } from "@playwright/test";

export type FileSubmenuId = "exchange" | "history" | "help";

/** Clicks the submenu's row and waits until its items are visible. The File menu must be open. */
export async function openFileSubmenu(page: Page, id: FileSubmenuId) {
  const trigger = page.locator(`[data-file-submenu-trigger="${id}"]`);
  await trigger.click();
  await expect(trigger).toHaveAttribute("aria-expanded", "true");
  await expect(page.locator(`[data-file-submenu-panel="${id}"]`)).toBeVisible();
}
