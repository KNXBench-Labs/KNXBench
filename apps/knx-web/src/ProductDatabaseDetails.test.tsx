/** Verifies literal metadata rendering with the exact fictional DB-to-HTTP fixture. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import { ProductDatabaseDetails } from "./ProductDatabaseDetails";
import { product } from "../e2e/fixtures/product-details";
import { setSetting } from "./settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "./uiLanguage";

const host = document.createElement("div");
document.body.append(host);
const root = createRoot(host);
afterEach(async () => {
  await act(async () => root.render(null));
  setSetting(UI_LANGUAGE_STORAGE_KEY, "en");
});

it.each(["en", "de"])("renders every exact DB scalar and placement with %s labels and no writes", async language => {
  setSetting(UI_LANGUAGE_STORAGE_KEY, language);
  const sections = product.catalog!.details!;
  await act(async () => root.render(<ProductDatabaseDetails sections={sections} />));
  expect(host.querySelectorAll("details")).toHaveLength(sections.length);
  expect([...host.querySelectorAll("dd")].map(node => node.textContent)).toEqual(sections.flatMap(section => section.fields.map(field => field.value)));
  expect(host.textContent).toContain(language === "de" ? "Busstrom (Quellliteral)" : "Bus current (source literal)");
  expect(host.textContent).toContain("Second placement");
  expect(host.textContent).toContain("FutureHardwareAttribute");
  expect(host.querySelectorAll("input, button, textarea")).toHaveLength(0);
});

it("preserves empty/false/zero and unknown scalar keys without executing markup", async () => {
  await act(async () => root.render(<ProductDatabaseDetails sections={[{ kind: "future", id: "unknown-ID", fields: [
    { key: "future_key", value: "<script>future</script>" }, { key: "bus_current", value: "0" },
    { key: "is_secure_enabled", value: "false" }, { key: "text", value: "" },
  ] }]} />));
  expect(host.textContent).toContain("future_key");
  expect([...host.querySelectorAll("dd")].map(node => node.textContent)).toEqual(["<script>future</script>", "0", "false", "(empty value)"]);
  expect(host.querySelector("script")).toBeNull();
});
