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
import time

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

# The publication fixture (L2): one application program that exercises every
# mapping rule measured against ETS's own conversion (docs/research/
# legacy-vd-mapping.md). Addresses are in the 0x4100 page; names are
# invented.
I, S, V, B = 1, 2, 3, 8  # column type codes: integer, small integer, text, binary
PARAMETER_COLUMNS = [
    (I, 4, "N", "PARAMETER_ID"), (I, 4, "Y", "PROGRAM_ID"), (I, 4, "Y", "PARAMETER_TYPE_ID"),
    (V, 50, "Y", "PARAMETER_NUMBER"), (V, 50, "Y", "PARAMETER_NAME"),
    (S, 2, "Y", "PARAMETER_LOW_ACCESS"), (S, 2, "Y", "PARAMETER_HIGH_ACCESS"),
    (V, 20, "Y", "PARENT_PARM_VALUE"), (I, 4, "Y", "PARAMETER_SIZE"),
    (I, 4, "Y", "PARAMETER_DISPLAY_ORDER"), (I, 4, "Y", "PARAMETER_ADDRESS"),
    (S, 2, "Y", "PARAMETER_BITOFFSET"), (V, 255, "Y", "PARAMETER_DESCRIPTION"),
    (I, 4, "Y", "PAR_PARAMETER_ID"), (I, 4, "Y", "PARAMETER_DEFAULT_LONG"),
]


def parameter(pid, ptype, number, name, low, high, parent_value, size, order, address,
              bit, description, parent, default):
    return [str(pid), "300", str(ptype), number, name, str(low), str(high), parent_value,
            str(size), str(order), address, bit, description, parent, default]


PROGRAM_DATABASE = document("ets.vd_", "virtual_device", [
    table(3, "manufacturer", [(I, 4, "N", "MANUFACTURER_ID"), (V, 50, "Y", "MANUFACTURER_NAME")],
          [["4242", "Marvin Test"]]),
    table(5, "ete_language", [(I, 4, "N", "LANGUAGE_ID"), (V, 50, "Y", "LANGUAGE_NAME"),
                              (S, 2, "Y", "DATABASE_LANGUAGE")],
          [["1031", "Deutsch", "1"], ["1033", "English", "0"]]),
    table(8, "mask", [(I, 4, "N", "MASK_ID"), (S, 2, "Y", "MASK_VERSION"),
                      (V, 10, "Y", "MASK_VERSION_NAME")],
          [["900", "1793", "7.1"]]),
    table(9, "hw_product", [(I, 4, "N", "PRODUCT_ID"), (I, 4, "Y", "MANUFACTURER_ID"),
                            (V, 50, "Y", "PRODUCT_NAME"), (S, 2, "Y", "PRODUCT_VERSION_NUMBER"),
                            (V, 30, "Y", "PRODUCT_SERIAL_NUMBER"), (I, 4, "Y", "BUS_CURRENT"),
                            (I, 4, "Y", "ORIGINAL_MANUFACTURER_ID")],
          [["100", "4242", "Heart of Gold Sensor", "1", "HG-1", "10", "4242"]]),
    table(10, "catalog_entry", [(I, 4, "N", "CATALOG_ENTRY_ID"), (I, 4, "Y", "PRODUCT_ID"),
                                (I, 4, "Y", "MANUFACTURER_ID"), (V, 20, "Y", "ORDER_NUMBER"),
                                (V, 50, "Y", "ENTRY_NAME"),
                                (I, 4, "Y", "ENTRY_WIDTH_IN_MILLIMETERS"), (S, 2, "Y", "DIN_FLAG")],
          [["200", "100", "4242", "MT-42", "Heart of Gold Sensor", "18", "1"]]),
    table(11, "application_program", [(I, 4, "N", "PROGRAM_ID"), (I, 4, "Y", "MASK_ID"),
                                      (V, 50, "Y", "PROGRAM_NAME"), (S, 2, "Y", "PROGRAM_VERSION"),
                                      (I, 4, "Y", "MANUFACTURER_ID"), (S, 2, "Y", "LINKABLE"),
                                      (S, 2, "Y", "PEI_TYPE"), (I, 4, "Y", "PROGRAM_TYPE"),
                                      (I, 4, "Y", "ORIGINAL_MANUFACTURER_ID"),
                                      (S, 2, "Y", "DEVICE_TYPE")],
          [["300", "900", "Improbability Drive", "22", "4242", "0", "0", "1", "4242", "7"]]),
    table(12, "product_to_program", [(I, 4, "N", "PROD2PROG_ID"), (I, 4, "Y", "PRODUCT_ID"),
                                     (I, 4, "Y", "PROGRAM_ID"), (V, 20, "Y", "REGISTRATION_NUMBER")],
          [["400", "100", "300", "42/2026"]]),
    table(6, "functional_entity", [(I, 4, "N", "FUNCTIONAL_ENTITY_ID"), (I, 4, "Y", "MANUFACTURER_ID"),
                                   (V, 50, "Y", "FUNCTIONAL_ENTITY_NAME"),
                                   (V, 20, "Y", "FUNCTIONAL_ENTITY_NUMB"),
                                   (I, 4, "Y", "FUN_FUNCTIONAL_ENTITY_ID")],
          [["50", "4242", "Sensors", "1", ""], ["51", "4242", "Presence", "1.1", "50"]]),
    table(13, "virtual_device", [(I, 4, "N", "VIRTUAL_DEVICE_ID"), (I, 4, "Y", "CATALOG_ENTRY_ID"),
                                 (I, 4, "Y", "PROGRAM_ID"), (V, 50, "Y", "VIRTUAL_DEVICE_NAME"),
                                 (I, 4, "Y", "FUNCTIONAL_ENTITY_ID")],
          [["500", "200", "300", "Heart of Gold Sensor", "51"]]),
    table(14, "parameter_atomic_type", [(I, 4, "N", "ATOMIC_TYPE_NUMBER"),
                                        (V, 50, "Y", "ATOMIC_TYPE_NAME")],
          [["0", "none"], ["1", "unsigned"], ["2", "signed"], ["4", "enum"]]),
    table(15, "parameter_type", [(I, 4, "N", "PARAMETER_TYPE_ID"), (I, 4, "Y", "ATOMIC_TYPE_NUMBER"),
                                 (I, 4, "Y", "PROGRAM_ID"), (V, 50, "Y", "PARAMETER_TYPE_NAME"),
                                 (I, 4, "Y", "PARAMETER_MINIMUM_VALUE"),
                                 (I, 4, "Y", "PARAMETER_MAXIMUM_VALUE"),
                                 (I, 4, "Y", "PARAMETER_TYPE_SIZE")],
          [["10", "0", "300", "t_none", "", "", "0"],
           ["11", "1", "300", "t_byte", "0", "255", "8"],
           ["12", "2", "300", "t_offset", "-10", "10", "8"],
           ["13", "4", "300", "t_switch", "", "", "1"],
           ["14", "4", "300", "t_mode", "", "", "8"]]),
    # Display order deliberately differs from value order.
    table(16, "parameter_list_of_values",
          [(I, 4, "Y", "PARAMETER_TYPE_ID"), (I, 4, "Y", "REAL_VALUE"),
           (V, 50, "Y", "DISPLAYED_VALUE"), (I, 4, "Y", "DISPLAY_ORDER"),
           (I, 4, "N", "PARAMETER_VALUE_ID")],
          [["13", "1", "On", "1", "600"], ["13", "0", "Off", "2", "601"],
           ["14", "0", "Mostly harmless", "1", "602"], ["14", "2", "Panic", "2", "603"],
           ["14", "1", "Towel", "3", "604"]]),
    table(17, "parameter", PARAMETER_COLUMNS, [
        # Page: atomic type 0 without a parent.
        parameter(1000, 10, "1000", "p_general", 2, 2, "", 0, 1, "", "", "General", "", ""),
        parameter(1001, 13, "1001", "p_mode", 2, 2, "", 1, 10, "16640", "7", "Mode", "1000", "1"),
        # One memory cell, two numbers: the group takes 1002's value and access,
        # 1003 overrides both.
        parameter(1002, 11, "1002", "p_level", 2, 2, "1", 8, 20, "16641", "0", "Level", "1001", "42"),
        parameter(1003, 11, "1003", "p_level_off", 0, 0, "0", 8, 30, "16641", "0", "Level (off)",
                  "1001", "7"),
        # One address, two types: a union.
        parameter(1004, 14, "1004", "p_reaction", 2, 2, "1", 8, 40, "16642", "0", "Reaction",
                  "1001", "2"),
        parameter(1005, 12, "1005", "p_offset", 2, 2, "0", 8, 50, "16642", "0", "Offset", "1001",
                  "-3"),
        # A heading: atomic type 0 with a parent.
        parameter(1006, 10, "1006", "p_heading", 2, 2, "1", 0, 15, "", "", "Details", "1001", ""),
        # A hidden root control without memory, and what it governs.
        parameter(1010, 13, "1010", "p_hidden", 0, 0, "", 1, 2000, "", "", "Hidden", "", "1"),
        parameter(1011, 11, "1011", "p_governed", 2, 2, "1", 8, 60, "16643", "0", "Governed",
                  "1010", "5"),
        parameter(2000, 10, "2000", "p_expert", 2, 2, "", 0, 100, "", "", "Expert", "", ""),
        parameter(2001, 12, "2001", "p_trim", 2, 2, "", 8, 110, "16644", "0", "Trim", "2000", "0"),
    ]),
    table(18, "object_type", [(I, 4, "N", "OBJECT_TYPE_CODE"), (V, 20, "Y", "OBJECT_TYPE_NAME"),
                              (I, 4, "Y", "LENGTH_IN_BIT")],
          [["0", "1 Bit", "1"], ["8", "2 Byte", "16"]]),
    table(19, "object_priority", [(I, 4, "N", "OBJECT_PRIORITY_CODE"),
                                  (V, 20, "Y", "OBJECT_PRIORITY_NAME")],
          [["3", "Low"]]),
    table(20, "communication_object",
          [(I, 4, "Y", "PROGRAM_ID"), (V, 50, "Y", "OBJECT_NAME"), (V, 50, "Y", "OBJECT_FUNCTION"),
           (S, 2, "Y", "OBJECT_READENABLED"), (S, 2, "Y", "OBJECT_WRITEENABLED"),
           (S, 2, "Y", "OBJECT_COMMENABLED"), (S, 2, "Y", "OBJECT_TRANSENABLED"),
           (I, 4, "Y", "OBJECT_DISPLAY_ORDER"), (V, 20, "Y", "PARENT_PARAMETER_VALUE"),
           (I, 4, "N", "OBJECT_ID"), (I, 4, "Y", "PARAMETER_ID"), (I, 4, "Y", "OBJECT_NUMBER"),
           (V, 50, "Y", "OBJECT_DESCRIPTION"), (S, 2, "Y", "OBJECT_TYPE"),
           (S, 2, "Y", "OBJECT_PRIORITY"), (S, 2, "Y", "OBJECT_UPDATEENABLED"),
           (I, 4, "Y", "OBJECT_UNIQUE_NUMBER"), (S, 2, "Y", "OBJECT_READONINITENABLED")],
          [["300", "Alarm", "Send", "0", "0", "1", "1", "1", "", "700", "", "0", "", "0", "3",
            "0", "10000", "0"],
           ["300", "Level", "Receive", "0", "1", "1", "0", "2", "1", "701", "1001", "1", "", "0",
            "3", "0", "10001", "0"],
           ["300", "Level", "Value", "1", "1", "1", "0", "3", "0", "702", "1001", "1", "", "8",
            "3", "1", "10002", "0"]]),
    table(21, "text_attribute", [(I, 4, "N", "TEXT_ATTRIBUTE_ID"), (I, 4, "Y", "LANGUAGE_ID"),
                                 (I, 4, "Y", "COLUMN_ID"), (I, 4, "Y", "ENTITY_ID"),
                                 (V, 255, "Y", "TEXT_ATTRIBUTE_TEXT")],
          [["800", "1033", "10", "1001", "Mode (en)"],
           ["801", "1033", "11", "600", "On (en)"],
           ["802", "1033", "20", "701", "Level (en)"],
           ["803", "1033", "22", "701", "Receive (en)"],
           ["804", "1033", "1", "200", "Heart of Gold Sensor (en)"],
           # Group members (ADR-0094 L2): a translation equal to the
           # representative's is dropped unless the member overrides the text.
           # 1003 and 702's function override it and keep theirs; 702's name
           # does not.
           ["806", "1033", "10", "1002", "Level (en)"],
           ["807", "1033", "10", "1003", "Level (en)"],
           ["808", "1033", "20", "702", "Level (en)"],
           ["809", "1033", "22", "702", "Receive (en)"],
           ["805", "1033", "99", "1001", "Unknown column"]]),
    # A table the mapping does not model: kept in the payload, reported.
    table(22, "s19_block", [(I, 4, "N", "BLOCK_ID"), (I, 4, "Y", "PROGRAM_ID"),
                            (B, 32767, "Y", "BLOCK_DATA")],
          [["900", "300", "00FF"]]),
    # A secret-class column (ADR-0094, design decision B-3): the non-empty
    # value, wrapped over a continuation line, is withheld from the stored
    # copy; the empty one has nothing to withhold.
    table(23, "device", [(I, 4, "N", "DEVICE_ID"), (V, 50, "Y", "DEVICE_BCU_PASSWORD"),
                         (V, 50, "Y", "DEVICE_NAME")],
          [["1", ["Zaphod42", "\\\\" + "Beeblebrox"], "Kept"], ["2", "", "Also kept"]]),
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
    # Both prompts can arrive in one read, so count them cumulatively. After
    # printing a prompt, zipcloak switches echo off with TCSAFLUSH, which
    # discards input that is already waiting; answer only once it has.
    output = b""
    for answered in range(2):
        while output.lower().count(b"password") <= answered:
            output += os.read(fd, 256)
        time.sleep(0.5)
        os.write(fd, password.encode() + b"\n")
    try:
        while os.read(fd, 256):
            pass
    except OSError:
        pass
    _, status = os.waitpid(pid, 0)
    if status != 0:
        raise SystemExit(f"zipcloak failed for {encrypted}")


def build_program():
    """Builds only the L2 publication fixture; the L1 archives keep their digests."""
    src = os.path.join(HERE, "src-vd-program")
    write(os.path.join(src, "MARVIN", "ets.vd_"), PROGRAM_DATABASE)
    plain = zip_member(src, "MARVIN/ets.vd_", "marvin-program-plain.vd4")
    zipcloak(plain, "marvin-program.vd4", PASSWORD)


def main():
    # `zipcloak` draws a random encryption header, so rebuilding an encrypted
    # archive changes its digest. Build only what is asked for.
    if sys.argv[1:] == ["program"]:
        build_program()
        return 0
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
