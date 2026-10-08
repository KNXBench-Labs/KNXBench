/** ADR-0094 (L3): route a product file to its installer; read legacy-password refusals. */

export type LegacyPasswordRefusal = "required" | "wrong" | "rememberedDoesNotFit" | "rememberedUnusable";

const REFUSAL_KINDS: Record<string, LegacyPasswordRefusal> = {
  legacyPasswordRequired: "required",
  legacyWrongPassword: "wrong",
  legacyRememberedPasswordDoesNotFit: "rememberedDoesNotFit",
  legacyRememberedPasswordUnusable: "rememberedUnusable",
};

function refusalKind(error: unknown): string | null {
  const { status, body } = (error ?? {}) as { status?: unknown; body?: { kind?: unknown } | null };
  if (status !== 422 || !body || typeof body.kind !== "string") return null;
  return body.kind;
}

/** `.vd3`, `.vd4` or `.vd5`: a legacy ETS3 product database by name. */
export function isLegacyProductFileName(name: string): boolean {
  return /\.vd[345]$/i.test(name);
}

/** The `422` kinds of `POST /api/catalog/install-legacy` that a password answers. */
export function legacyPasswordRefusal(error: unknown): LegacyPasswordRefusal | null {
  const kind = refusalKind(error);
  return kind === null ? null : REFUSAL_KINDS[kind] ?? null;
}

/** The package installer recognised a legacy product database (by name or content). */
export function isLegacyProductRefusal(error: unknown): boolean {
  return refusalKind(error) === "legacyProductDatabase";
}

export type ProductInstallOutcome<P, L> = { kind: "package"; report: P } | { kind: "legacy"; report: L };

export interface ProductInstallers<P, L> {
  installPackage: (file: File) => Promise<P>;
  installLegacy: (file: File, password?: string, remember?: boolean) => Promise<L>;
}

/**
 * A `.vd3`–`.vd5` goes to the legacy installer; anything else to the package
 * installer, and a legacy database it recognises (a renamed one) goes on to
 * the legacy installer. Every other failure is passed through unchanged.
 */
export async function installProductFile<P, L>(
  file: File,
  installers: ProductInstallers<P, L>,
  legacy: { password?: string; remember?: boolean } = {},
): Promise<ProductInstallOutcome<P, L>> {
  const viaLegacy = async () => ({
    kind: "legacy" as const,
    report: await installers.installLegacy(file, legacy.password, legacy.remember ?? false),
  });
  if (isLegacyProductFileName(file.name)) return viaLegacy();
  try {
    return { kind: "package", report: await installers.installPackage(file) };
  } catch (error) {
    if (isLegacyProductRefusal(error)) return viaLegacy();
    throw error;
  }
}
