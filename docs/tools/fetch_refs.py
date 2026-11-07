#!/usr/bin/env python3
"""fetch_refs.py - download the reference documents and test suites (run by the USER, network needed once).

Usage:
    python docs/tools/fetch_refs.py              # everything
    python docs/tools/fetch_refs.py --only docs  # Yacht.txt + manuals
    python docs/tools/fetch_refs.py --only tests # SingleStepTests/m68000 (127 files, ~137 MB)

Python 3.8+, standard library only.  Existing files are kept (use --force to download again).
"""
import argparse
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = [
    # primary mirror first, then the Wayback Machine copy (raw file, "id_" flag) when it is down (502)
    ("docs/ref/Yacht.txt",
     ("http://nemesis.hacking-cult.org/MegaDrive/Documentation/Yacht.txt",
      "https://web.archive.org/web/2020id_/http://nemesis.hacking-cult.org/MegaDrive/Documentation/Yacht.txt"),
     10000),
    ("docs/ref/M68000UM_Rev8.pdf",
     "https://cdn.hackaday.io/files/1805367724052224/M68000UM_AD_M68000_Microprocessor_Users_Manual_Rev8_1993.pdf", 100000),
    ("docs/ref/M68000PM.pdf", "https://www.eecg.utoronto.ca/~moshovos/ECE243-07/M68000PM.pdf", 100000),
]
SST_RAW = "https://raw.githubusercontent.com/SingleStepTests/m68000/main/v1/"


def get(url, dest, min_size, force):
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size >= min_size and not force:
        print("keep   %s" % dest.relative_to(ROOT))
        return True
    for u in (url if isinstance(url, tuple) else (url,)):
        try:
            req = urllib.request.Request(u, headers={"User-Agent": "puce-m68k-fetch"})
            with urllib.request.urlopen(req, timeout=120) as r:
                data = r.read()
        except Exception as e:  # noqa
            print("FAIL   %s (%s)" % (u, e))
            continue
        if len(data) < min_size:
            print("FAIL   %s (only %d bytes)" % (u, len(data)))
            continue
        dest.write_bytes(data)
        print("ok     %s (%d bytes)" % (dest.relative_to(ROOT), len(data)))
        return True
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", choices=["docs", "tests"])
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()
    ok = True
    if a.only in (None, "docs"):
        for rel, url, mn in DOCS:
            ok &= get(url, ROOT / rel, mn, a.force)
    if a.only in (None, "tests"):
        names = (ROOT / "tests/singlestep/FILES.txt").read_text().split()
        for n in names:
            ok &= get(SST_RAW + n, ROOT / "tests/singlestep" / n, 100000, a.force)
    print("DONE" if ok else "SOME DOWNLOADS FAILED (rerun, or download by hand into the paths above)")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
