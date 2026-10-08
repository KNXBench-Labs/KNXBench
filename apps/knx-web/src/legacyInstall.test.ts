/** Routing a product file to the right installer and reading legacy-password refusals. */
import { describe, expect, it, vi } from "vitest";
import {
  installProductFile,
  isLegacyProductFileName,
  isLegacyProductRefusal,
  legacyPasswordRefusal,
} from "./legacyInstall";

function refusal(kind: string, status = 422) {
  return Object.assign(new Error(kind), { status, body: { error: kind, kind } });
}

const file = (name: string) => new File([new Uint8Array([1, 2, 3])], name);

describe("isLegacyProductFileName", () => {
  it("knows the three legacy product database extensions in any case", () => {
    for (const name of ["a.vd3", "b.VD4", "c.Vd5"]) expect(isLegacyProductFileName(name)).toBe(true);
    for (const name of ["a.knxprod", "b.pr5", "c.vd2", "vd4", "d.vd4.zip"]) {
      expect(isLegacyProductFileName(name)).toBe(false);
    }
  });
});

describe("legacyPasswordRefusal", () => {
  it("maps each server kind and ignores everything else", () => {
    expect(legacyPasswordRefusal(refusal("legacyPasswordRequired"))).toBe("required");
    expect(legacyPasswordRefusal(refusal("legacyWrongPassword"))).toBe("wrong");
    expect(legacyPasswordRefusal(refusal("legacyRememberedPasswordDoesNotFit"))).toBe("rememberedDoesNotFit");
    expect(legacyPasswordRefusal(refusal("legacyRememberedPasswordUnusable"))).toBe("rememberedUnusable");
    expect(legacyPasswordRefusal(refusal("legacyWrongPassword", 400))).toBeNull();
    expect(legacyPasswordRefusal(refusal("projectPasswordRequired"))).toBeNull();
    expect(legacyPasswordRefusal(new Error("plain"))).toBeNull();
    expect(legacyPasswordRefusal(null)).toBeNull();
  });
});

describe("isLegacyProductRefusal", () => {
  it("is the package installer's 422 legacyProductDatabase only", () => {
    expect(isLegacyProductRefusal(refusal("legacyProductDatabase"))).toBe(true);
    expect(isLegacyProductRefusal(refusal("legacyProductDatabase", 400))).toBe(false);
    expect(isLegacyProductRefusal(refusal("legacyWrongPassword"))).toBe(false);
  });
});

describe("installProductFile", () => {
  const packageReport = { packageSha256: "p" };
  const legacyReport = { namespace: "LX00000000" };

  it("sends a .vd4 straight to the legacy installer with what was given", async () => {
    const installPackage = vi.fn();
    const installLegacy = vi.fn().mockResolvedValue(legacyReport);
    const outcome = await installProductFile(file("m.vd4"), { installPackage, installLegacy }, {
      password: "pw",
      remember: true,
    });
    expect(outcome).toEqual({ kind: "legacy", report: legacyReport });
    expect(installPackage).not.toHaveBeenCalled();
    expect(installLegacy).toHaveBeenCalledWith(expect.any(File), "pw", true);
  });

  it("installs a package, and retries a renamed legacy database on the legacy route", async () => {
    const installLegacy = vi.fn().mockResolvedValue(legacyReport);
    const ok = await installProductFile(file("m.knxprod"), {
      installPackage: vi.fn().mockResolvedValue(packageReport),
      installLegacy,
    });
    expect(ok).toEqual({ kind: "package", report: packageReport });
    expect(installLegacy).not.toHaveBeenCalled();

    const renamed = await installProductFile(file("renamed.knxprod"), {
      installPackage: vi.fn().mockRejectedValue(refusal("legacyProductDatabase")),
      installLegacy,
    });
    expect(renamed).toEqual({ kind: "legacy", report: legacyReport });
    expect(installLegacy).toHaveBeenCalledWith(expect.any(File), undefined, false);
  });

  it("passes every other failure through unchanged", async () => {
    const failure = refusal("somethingElse");
    await expect(installProductFile(file("m.knxprod"), {
      installPackage: vi.fn().mockRejectedValue(failure),
      installLegacy: vi.fn(),
    })).rejects.toBe(failure);
    const wrong = refusal("legacyWrongPassword");
    await expect(installProductFile(file("m.vd3"), {
      installPackage: vi.fn(),
      installLegacy: vi.fn().mockRejectedValue(wrong),
    })).rejects.toBe(wrong);
  });
});
