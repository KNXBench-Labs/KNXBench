/** Verifies new-project UI creation for every style with all API requests locally intercepted. */

import { expect, test } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";

const styles = ["ThreeLevel", "TwoLevel", "Free"] as const;

for (const style of styles) {
  test(`creates a ${style} project from scratch and opens its unassigned catalog`, async ({
    page,
  }) => {
    const projectName = `Browser ${style}`;
    const installationName = `Installation ${style}`;
    const unexpected: string[] = [];
    let current: ProjectTree = {
      schema_version: 11, errors: 0, warnings: 0, can_undo: false,
      can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
      server_incarnation: "new-project-fixture", snapshot_revision: 0,
      installations: [],
    };
    await page.route("**/api/**", (route) => {
      const path = new URL(route.request().url()).pathname;
      const method = route.request().method();
      let body: unknown;
      if (path === "/api/auth/status" && method === "GET") {
        body = { required: false, authenticated: true };
      } else if (path === "/api/settings" && method === "GET") {
        body = { schemaVersion: 1, status: "ok", settings: {} };
      } else if (path === "/api/project" && method === "GET") {
        body = current;
      } else if (path === "/api/bus/discover" && method === "POST") {
        // The sidebar's automatic search is intercepted, never a real UDP scan.
        body = { interfaces: [] };
      } else if (path === "/api/project/new" && method === "POST") {
        const request = route.request().postDataJSON();
        current = {
          ...current, group_address_style: request.groupAddressStyle, snapshot_revision: 1,
          installations: [{ id: 1, name: request.installationName, topology: [],
            buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
        };
        body = current;
      } else if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items"].includes(path)) {
        body = [];
      } else {
        unexpected.push(`${method} ${path}`);
        return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
      }
      return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    });

    await page.goto("/");
    await page.getByRole("button", { name: "New project…" }).click();

    const dialog = page.getByRole("dialog", { name: "New project" });
    await expect(dialog).toBeVisible();
    await expect(
      dialog.getByText(
        "An empty project with one installation. Nothing is written to disk until you save it.",
      ),
    ).toBeVisible();
    await expect(dialog.getByLabel("Project name")).toHaveValue("Untitled project");
    await expect(dialog.getByLabel("Installation name")).toHaveValue("Installation 1");
    await expect(dialog.getByLabel("Project language")).toHaveValue("en");

    await dialog.getByLabel("Project name").fill(projectName);
    await dialog.getByLabel("Installation name").fill(installationName);
    await dialog.getByLabel("Project language").selectOption("en");
    await dialog.getByLabel("Group address style").selectOption(style);

    const responsePromise = page.waitForResponse(
      (response) =>
        response.url().endsWith("/api/project/new") &&
        response.request().method() === "POST",
    );
    await dialog.getByRole("button", { name: "Create project" }).click();
    const response = await responsePromise;

    expect(response.status()).toBe(200);
    expect(response.request().postDataJSON()).toEqual({
      name: projectName,
      installationName,
      language: "en",
      groupAddressStyle: style,
      discardChanges: false,
    });

    const tree = (await response.json()) as {
      group_address_style: string;
      installations: Array<{
        name: string;
        topology: unknown[];
        unassigned: unknown[];
      }>;
    };
    expect(tree.group_address_style).toBe(style);
    expect(tree.installations).toHaveLength(1);
    expect(tree.installations[0]).toMatchObject({
      name: installationName,
      topology: [],
      unassigned: [],
    });

    await expect(dialog).toBeHidden();
    await expect(
      page.locator("button.tree-label").filter({ hasText: installationName }),
    ).toBeVisible();

    await page.getByRole("button", { name: "Project", exact: true }).click();
    await expect(page.getByLabel("Group address style")).toHaveValue(style);

    await page.getByRole("button", { name: "+ Add device" }).click();
    await expect(page.getByRole("region", { name: "Device catalog" })).toBeVisible();
    expect(unexpected).toEqual([]);
  });
}
