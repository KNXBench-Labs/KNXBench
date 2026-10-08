#!/usr/bin/env python3
"""Rebuilds the synthetic legacy EX-IM fixtures (README.md has the recipe and digests).

Everything here is invented for KNXBench: the manufacturer "Marvin Test", its
products and ids, and the public test password `marvin-synthetic`. No byte
comes from a manufacturer database.

Archiving and encryption are done by Info-ZIP `zip` and `zipcloak`
(independent tools; the repository has no ZipCrypto writer). `zipcloak`
encrypts a finished archive in place, which yields the layout every observed
real legacy file has: flags 0x0001, no data descriptor, CRC-32 check byte.
`zip -P` would stream instead (bit 3 plus a descriptor). `zipcloak` reads the
password from a terminal only, so a pseudo-terminal feeds it.
"""

import os
import pty
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PASSWORD = "marvin-synthetic"
SEPARATOR = "-" * 37


def table(tid, name, cols, rows):
    out = [SEPARATOR, f"T {tid} {name}"]
    for i, (ctype, size, nullable, cname) in enumerate(cols, 1):
        out.append(f"C{i} T{tid} {ctype} {size} {nullable} {cname}")
    for r, vals in enumerate(rows, 1):
        out.append(f"R {r} T {tid} {name}")
        assert len(vals) == len(cols)
        for v in vals:
            out.extend(v if isinstance(v, list) else [v])
    return out


def document(member, kind, tables):
    lines = ["EX-IM", f"N C:\\synthetic\\{member}", "K ETS3", "K ",
             "D 2026-10-08 09:00:00", "V 6.2", f"H {kind}"]
    for t in tables:
        lines += t
    lines.append("XXX")
    return ("\r\n".join(lines) + "\r\n").encode("cp1252")


# An 80-character first line plus two `\\` continuation lines, the way the
# observed files wrap long hexadecimal values.
HEX_VALUE = ["0123456789ABCDEF" * 5, "\\\\" + "FEDCBA9876543210" * 5, "\\\\" + "00FF"]

PRODUCT_DATABASE = document("ets.vd_", "virtual_device", [
    table(3, "manufacturer", [(1, 4, "N", "MANUFACTURER_ID"), (3, 50, "Y", "MANUFACTURER_NAME")],
          [["4242", "Marvin Test"]]),
    table(8, "mask", [(1, 4, "N", "MASK_ID"), (2, 2, "Y", "MASK_VERSION"),
                      (3, 10, "Y", "MASK_VERSION_NAME")],
          [["900", "1793", "7.1"]]),
    table(9, "hw_product", [(1, 4, "N", "PRODUCT_ID"), (1, 4, "Y", "MANUFACTURER_ID"),
                            (3, 50, "Y", "PRODUCT_NAME")],
          [["100", "4242", "Heart of Gold Sensor"]]),
    table(10, "catalog_entry", [(1, 4, "N", "CATALOG_ENTRY_ID"), (1, 4, "Y", "PRODUCT_ID"),
                                (1, 4, "Y", "MANUFACTURER_ID"), (3, 20, "Y", "ORDER_NUMBER"),
                                (3, 50, "Y", "ENTRY_NAME")],
          [["200", "100", "4242", "MT-42", "Heart of Gold Sensor"],
           ["201", "100", "4242", "MT-42-B", "Heart of Gold Sensor b\xe9ta"]]),
    table(11, "application_program", [(1, 4, "N", "PROGRAM_ID"), (1, 4, "Y", "MASK_ID"),
                                      (3, 50, "Y", "PROGRAM_NAME"), (2, 2, "Y", "PROGRAM_VERSION"),
                                      (1, 4, "Y", "MANUFACTURER_ID"),
                                      (8, 32767, "Y", "EEPROM_DATA")],
          [["300", "900", "Improbability Drive", "16", "4242", HEX_VALUE]]),
    table(12, "virtual_device", [(1, 4, "N", "VIRTUAL_DEVICE_ID"), (1, 4, "Y", "CATALOG_ENTRY_ID"),
                                 (1, 4, "Y", "PROGRAM_ID"), (3, 50, "Y", "VIRTUAL_DEVICE_NAME")],
          [["400", "200", "300", "Heart of Gold Sensor"],
           ["401", "201", "300", "Heart of Gold Sensor b\xe9ta"]]),
    # Values that look like separators must stay values.
    table(13, "parameter", [(1, 4, "N", "PARAMETER_ID"), (3, 50, "Y", "PARAMETER_NAME"),
                            (3, 50, "Y", "PARAMETER_DESCRIPTION")],
          [["500", "Page Towel", "----"], ["501", "Answer", "-"]]),
])

PROJECT_EXPORT = document("ets.pr_", "project", [
    table(4, "project", [(1, 4, "N", "PROJECT_ID"), (3, 50, "Y", "PROJECT_NAME"),
                         (3, 20, "Y", "PROJECT_PASSWORD")],
          [["1", "Synthetic Project", ""]]),
    table(7, "manufacturer", [(1, 4, "N", "MANUFACTURER_ID"), (3, 50, "Y", "MANUFACTURER_NAME")],
          [["4242", "Marvin Test"]]),
    table(8, "application_program", [(1, 4, "N", "PROGRAM_ID")], []),
])


def write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)


def zip_member(source_dir, member, archive):
    out = os.path.join(HERE, archive)
    if os.path.exists(out):
        os.remove(out)
    env = dict(os.environ, TZ="UTC")
    subprocess.run(["touch", "-t", "202610080900", os.path.join(source_dir, member)],
                   check=True, env=env)
    subprocess.run(["zip", "-q", "-X", "-D", out, member], cwd=source_dir, check=True, env=env)
    return out


def zipcloak(plain, encrypted, password):
    """Runs `zipcloak -O encrypted plain`, answering both password prompts."""
    target = os.path.join(HERE, encrypted)
    if os.path.exists(target):
        os.remove(target)
    pid, fd = pty.fork()
    if pid == 0:
        os.execvp("zipcloak", ["zipcloak", "-O", target, plain])
    output = b""
    for _ in range(2):
        while b"password" not in output:
            output += os.read(fd, 256)
        output = b""
        os.write(fd, password.encode() + b"\n")
    try:
        while os.read(fd, 256):
            pass
    except OSError:
        pass
    _, status = os.waitpid(pid, 0)
    if status != 0:
        raise SystemExit(f"zipcloak failed for {encrypted}")


def main():
    vd_dir = os.path.join(HERE, "src-vd")
    pr_dir = os.path.join(HERE, "src-pr")
    write(os.path.join(vd_dir, "MARVIN", "ets.vd_"), PRODUCT_DATABASE)
    write(os.path.join(pr_dir, "MARVIN", "ets.pr_"), PROJECT_EXPORT)
    plain_vd = zip_member(vd_dir, "MARVIN/ets.vd_", "marvin-plain.vd4")
    zipcloak(plain_vd, "marvin-encrypted.vd4", PASSWORD)
    plain_pr = zip_member(pr_dir, "MARVIN/ets.pr_", "marvin-project-plain.tmp.zip")
    zipcloak(plain_pr, "marvin-project.pr5", PASSWORD)
    os.remove(plain_pr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
