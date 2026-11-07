#!/usr/bin/env python3
"""Split docs/ref/Yacht.txt into one file per mnemonic: docs/specs/instr/<MNEMONIC>.md

Usage:
  python docs/tools/split_yacht.py           regenerate everything
  python docs/tools/split_yacht.py --check   verify every mnemonic has a non-empty file
  python docs/tools/split_yacht.py --list    show the title found for every table

Method (nothing is guessed):
  * a "table" starts at a dashes line followed by a header row containing "Exec Time"
    (or "Periods"); it ends at the next table / "****" section banner.
  * the table title is the text left of the first "|" on the 2 header lines.
  * every mnemonic named in a title (through the ALIAS table below) gets the whole table,
    copied verbatim (non-ASCII bytes replaced by '?', trailing blanks removed).
  * the "Line xxxx" banners are copied into the file of the mnemonics they list.
  * mnemonics never named are written to docs/specs/instr/MISSING.txt with the reason.
Python 3.8+, standard library only.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "docs" / "ref" / "Yacht.txt"
TSV = ROOT / "docs" / "specs" / "data" / "instructions.tsv"
OUT = ROOT / "docs" / "specs" / "instr"

# name found in a title (upper case) -> mnemonics of instructions.tsv
ALIAS = {
    "ADDI": ["ADD"], "ADDQ": ["ADD"], "SUBI": ["SUB"], "SUBQ": ["SUB"],
    "CMPI": ["CMP"], "CMPM": ["CMP"], "ANDI": ["AND"], "ORI": ["OR"], "EORI": ["EOR"],
    "BRA": ["Bcc"], "BCC": ["Bcc"], "DBCC": ["DBcc"], "SCC": ["Scc"],
    "UNLK": ["UNLINK"], "UNLNK": ["UNLINK"], "MOVEQ": ["MOVEQ"],
    "ILLEGAL": ["ILLEGAL_LINEA", "ILLEGAL_LINEF"],
}
# phrases of a title -> mnemonics (checked before the word scan)
PHRASES = [
    (r"MOVE\s+FROM\s+SR", ["MOVEfromSR"]),
    (r"MOVE\s+FROM\s+USP", ["MOVEfromUSP"]),
    (r"MOVE\s+TO\s+USP", ["MOVEtoUSP"]),
    (r"MOVE\s+USP", ["MOVEfromUSP", "MOVEtoUSP"]),
    (r"MOVE\s+TO\s+CCR,\s*TO\s+SR", ["MOVEtoCCR", "MOVEtoSR"]),
    (r"MOVE\s+TO\s+CCR", ["MOVEtoCCR"]),
    (r"MOVE\s+TO\s+SR", ["MOVEtoSR"]),
]
# banner lines of sections without table -> mnemonics (reserved line A / line F)
RESERVED = {"LINE 1010": ["ILLEGAL_LINEA"], "LINE 1111": ["ILLEGAL_LINEF"]}
EXCEPTION_USERS = ["ILLEGAL_LINEA", "ILLEGAL_LINEF", "CHK", "TRAP", "TRAPV", "DIVS", "DIVU"]


def clean(s):
    return "".join(c if ord(c) < 128 else "?" for c in s).rstrip()


def load_lines():
    raw = SRC.read_bytes().decode("latin-1")
    return [clean(x) for x in raw.splitlines()]


def mnemonics():
    rows = [r.split("\t") for r in TSV.read_text(encoding="utf-8").splitlines()[1:] if r.strip()]
    out = []
    for r in rows:
        if r[1] not in out:
            out.append(r[1])
    return out


def find_tables(lines):
    """-> list of (start, end, title) with 0-based [start, end)"""
    starts = []
    for i in range(len(lines) - 2):
        if re.match(r"^-{30,}", lines[i]) and re.search(r"Exec Time|Periods", lines[i + 1]):
            starts.append(i)
    banners = [i for i, l in enumerate(lines) if l.startswith("*" * 30)]
    tables = []
    for k, s in enumerate(starts):
        nxt = [starts[k + 1]] if k + 1 < len(starts) else [len(lines)]
        nxt += [b for b in banners if b > s]
        e = min(nxt)
        cells = []
        for j in range(s + 1, min(s + 6, e)):
            if lines[j].startswith("---") or "|" not in lines[j]:
                break
            cells.append(lines[j].split("|")[0].strip())
        title = " ".join(cells).strip()
        title = re.sub(r"\s+", " ", title)
        tables.append((s, e, title))
    return tables


def names_in_title(title, known):
    t = title.upper()
    found = []

    def add(m):
        if m in known and m not in found:
            found.append(m)
    for pat, ms in PHRASES:
        if ms and re.search(pat, t):
            for m in ms:
                add(m)
            t = re.sub(pat, " ", t)
    # "ORI, ANDI, EORI to CCR, to SR" / "to CCR" family
    if re.search(r"TO\s+CCR", t) and re.search(r"TO\s+SR", t):
        for base in ("ORI", "ANDI", "EORI"):
            if re.search(r"\b%s\b" % base, t):
                add(base + "toCCR")
                add(base + "toSR")
                t = re.sub(r"\b%s\b" % base, " ", t)
        t = re.sub(r"TO\s+(CCR|SR)", " ", t)
    for w in re.findall(r"[A-Z][A-Z0-9_]*", t):
        for m in ALIAS.get(w, [w] if w in {k.upper() for k in known} else []):
            add(m)
        for k in known:
            if k.upper() == w:
                add(k)
    return found


def banners(lines):
    """-> list of (start, end) 0-based of every '****' banner block (3 lines + frame)"""
    res = []
    idx = [i for i, l in enumerate(lines) if l.startswith("*" * 30)]
    for a, b in zip(idx, idx[1:]):
        if b - a <= 6 and any(x.strip().upper().startswith("LINE ") for x in lines[a:b]):
            res.append((a, b + 1))
    return res


def build():
    lines = load_lines()
    known = mnemonics()
    tables = find_tables(lines)
    per = {m: [] for m in known}          # mnemonic -> list of (label, start, end)
    notes = []
    for s, e, title in tables:
        if title.upper().startswith("PERIODS") or "|" in lines[s + 1] and "Periods" in lines[s + 1]:
            if s > 1500:
                continue
            for m in EXCEPTION_USERS:
                if m in per:
                    per[m].append(("exception table (%s)" % (title or "no title"), s, e))
            continue
        ms = names_in_title(title, known)
        if not ms:
            notes.append("table at line %d (title %r) names no known mnemonic" % (s + 1, title))
        for m in ms:
            per[m].append((title, s, e))
    # section banners
    for a, b in banners(lines):
        block = lines[a:b]
        text = " ".join(block).upper()
        key = re.search(r"LINE [01]{4}", text)
        names = set()
        for w in re.findall(r"[A-Z][A-Z0-9_]*", " ".join(block[1:]).upper()):
            for m in ALIAS.get(w, [w]):
                if m in {k.upper() for k in known}:
                    names.update(k for k in known if k.upper() == m.upper())
        if key and key.group(0) in RESERVED:
            names.update(RESERVED[key.group(0)])
        for m in names:
            if m in per:
                per[m].insert(0, ("section banner", a, b))
    OUT.mkdir(parents=True, exist_ok=True)
    missing = []
    for m in known:
        parts = per[m]
        has_table = any(lab not in ("section banner",) for lab, _, _ in parts)
        if not has_table:
            reason = "no table of Yacht.txt names it"
            if parts:
                reason += " (only a section banner lists it)"
            missing.append((m, reason))
            f = OUT / (m + ".md")
            if f.exists():
                f.unlink()
            continue
        seen = set()
        out = ["# %s" % m, "",
               "Source: docs/ref/Yacht.txt (verbatim copy; non-ASCII characters replaced by '?').", ""]
        for lab, s, e in parts:
            if (s, e) in seen:
                continue
            seen.add((s, e))
            out.append("## %s (lines %d-%d)" % (lab or "untitled", s + 1, e))
            out.append("")
            out.append("```text")
            blk = lines[s:e]
            while blk and not blk[-1].strip():
                blk.pop()
            out.extend(blk)
            out.append("```")
            out.append("")
        (OUT / (m + ".md")).write_text("\n".join(out) + "\n", encoding="utf-8", newline="\n")
    mt = ["Mnemonics of instructions.tsv without a table in docs/ref/Yacht.txt (nothing was guessed):", ""]
    mt += ["%s: %s" % x for x in missing]
    if notes:
        mt += ["", "Tables whose title names no known mnemonic:"] + notes
    (OUT / "MISSING.txt").write_text("\n".join(mt) + "\n", encoding="utf-8", newline="\n")
    print("wrote %d files, %d missing (see docs/specs/instr/MISSING.txt)" % (len(known) - len(missing), len(missing)))
    for n in notes:
        print("note:", n)
    return 0


def check():
    known = mnemonics()
    miss_txt = OUT / "MISSING.txt"
    listed = set()
    if miss_txt.exists():
        for l in miss_txt.read_text(encoding="utf-8").splitlines():
            if ": " in l and not l.startswith(("table", "Mnemonics")):
                listed.add(l.split(": ")[0])
    bad = []
    for m in known:
        f = OUT / (m + ".md")
        if f.exists() and f.stat().st_size > 0:
            continue
        if m in listed:
            print("missing (listed in MISSING.txt): %s" % m)
            continue
        bad.append(m)
    if bad:
        print("ERROR: no file and not listed: %s" % ", ".join(bad))
        return 1
    print("check ok: %d mnemonics, %d listed as missing" % (len(known), len(listed)))
    return 0


def list_titles():
    for s, e, t in find_tables(load_lines()):
        print("%5d-%5d  %-40s -> %s" % (s + 1, e, t, names_in_title(t, mnemonics())))
    return 0


if __name__ == "__main__":
    if "--check" in sys.argv:
        sys.exit(check())
    if "--list" in sys.argv:
        sys.exit(list_titles())
    sys.exit(build())
