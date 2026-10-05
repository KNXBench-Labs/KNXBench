/** AR08: a protected ETS project asks for its password in the real app; the secret leaves once. */

import { expect, test, type Page } from "@playwright/test";
import type { ProjectTree } from "../src/bindings/ProjectTree";

const SECRET = "fixture-s3cret";

async function serve(page: Page, answers: Array<"required" | "wrong" | "ok">) {
  const imports: Array<Record<string, unknown>> = [];
  const leaks: string[] = [];
  const unexpected: string[] = [];
  let current: ProjectTree | null = null;
  page.on("request", (request) => {
    // Every request the browser sends, wherever it goes: the password may
    // appear only in the body of an import POST.
    const url = request.url();
    const body = request.postData() ?? "";
    const isImport = new URL(url).pathname === "/api/project/import";
    if ((url.includes(SECRET) || body.includes(SECRET)) && !isImport) leaks.push(`${request.method()} ${url}`);
  });
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const ok = (body: unknown) => route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return ok({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return ok({ schemaVersion: 1, status: "ok", settings: {} });
    if (path === "/api/project" && method === "GET") {
      return current ? ok(current) : route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"no project open"}' });
    }
    if (path === "/api/bus/discover" && method === "POST") return ok({ interfaces: [] });
    if (path === "/api/project/load-progress") return ok(null);
    if (path === "/api/fs/list") return ok([{ name: "villa.knxproj", is_dir: false }]);
    if (method === "GET" && ["/api/product-languages", "/api/catalog/manufacturers", "/api/catalog/items", "/api/session-log"].includes(path)) return ok([]);
    if (path === "/api/project/import" && method === "POST") {
      imports.push(request.postDataJSON());
      const answer = answers.shift();
      if (answer === "ok") {
        current = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
          group_address_style: "ThreeLevel", server_incarnation: "password-fixture", snapshot_revision: 1,
          installations: [{ id: 1, name: "Villa", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }] };
        return ok(current);
      }
      const kind = answer === "wrong" ? "projectPasswordWrong" : "projectPasswordRequired";
      return route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: "project password", kind }) });
    }
    unexpected.push(`${method} ${path}`);
    return route.fulfill({ status: 404, contentType: "application/json", body: '{"error":"local fixture only"}' });
  });
  await page.goto("/");
  await page.getByText("Import ETS project").click();
  await page.getByRole("button", { name: "villa.knxproj" }).click();
  return { imports, leaks, unexpected };
}

test("asks for the password, retries once with it, and loads the project", async ({ page }) => {
  const { imports, leaks } = await serve(page, ["required", "wrong", "ok"]);
  const dialog = page.getByRole("dialog", { name: "Project password for villa.knxproj" });
  await expect(dialog).toContainText("password-protected");
  const field = dialog.getByLabel("Project password");
  await expect(field).toHaveAttribute("type", "password");
  await field.fill("guess");
  await dialog.getByRole("button", { name: "Import" }).click();
  await expect(dialog).toContainText("was not accepted");
  await field.fill(SECRET);
  await dialog.getByRole("button", { name: "Import" }).click();
  await expect(page.locator("button.tree-label", { hasText: "Villa" })).toBeVisible();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(imports.map((body) => body.password ?? null)).toEqual([null, "guess", SECRET]);
  expect(imports.every((body) => body.path === "villa.knxproj")).toBe(true);
  expect(leaks).toEqual([]);
  expect(await page.evaluate((secret) => JSON.stringify(localStorage).includes(secret) || JSON.stringify(sessionStorage).includes(secret), SECRET)).toBe(false);
});

test("cancel leaves no project, no error banner and no further request", async ({ page }) => {
  const { imports } = await serve(page, ["required"]);
  const dialog = page.getByRole("dialog", { name: "Project password for villa.knxproj" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByText("Could not load")).toHaveCount(0);
  expect(imports).toHaveLength(1);
});
