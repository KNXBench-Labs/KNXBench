import { expect, test } from "@playwright/test";

const styles = ["ThreeLevel", "TwoLevel", "Free"] as const;

for (const style of styles) {
  test(`creates a ${style} project from scratch and opens its unassigned catalog`, async ({
    page,
  }) => {
    const projectName = `Browser ${style}`;
    const installationName = `Installation ${style}`;

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
    await dialog.getByLabel("Project language").fill("en");
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
    await expect(page.locator(".inspector-facts")).toContainText(style);

    await page.getByRole("button", { name: "+ Add device" }).click();
    await expect(page.getByRole("dialog", { name: "Device catalog" })).toBeVisible();
  });
}
