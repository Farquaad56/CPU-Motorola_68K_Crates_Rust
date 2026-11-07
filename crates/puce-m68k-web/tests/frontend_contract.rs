//! Contract between the page `docs/frontend/index.html` and the protocol (spec 31).

use std::collections::BTreeSet;

fn page() -> String {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/frontend/index.html"
    );
    std::fs::read_to_string(p).expect("docs/frontend/index.html")
}

/// The inline script (the last `<script>` block without `src`).
fn script(html: &str) -> &str {
    let start = html.rfind("<script>").expect("inline script") + "<script>".len();
    let end = html[start..].find("</script>").expect("end of script") + start;
    &html[start..end]
}

/// Keys of the translation table of `lang`.
fn keys(html: &str, lang: &str) -> BTreeSet<String> {
    let open = format!("  {lang}: {{");
    let start = html.find(&open).unwrap_or_else(|| panic!("table {lang}")) + open.len();
    let end = html[start..].find("\n  }").expect("end of table") + start;
    html[start..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let (k, v) = l.split_once(':')?;
            (k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && v.trim_start().starts_with('"'))
            .then(|| k.to_string())
        })
        .collect()
}

#[test]
fn e13_06_title() {
    let h = page();
    assert!(h.contains("<title>Motorola 68000 Emulator</title>"));
    assert!(h.contains("<h1"));
}

#[test]
fn e13_06_tailwind() {
    let h = page();
    assert!(h.contains("https://cdn.tailwindcss.com"));
    assert!(h.contains("<meta name=\"viewport\""));
}

#[test]
fn e13_06_ws_fields() {
    let h = page();
    // fields of spec 31 section 2 and 3 shown by the page
    for f in [
        "status",
        "instrs",
        "clocks",
        "ips",
        "mhz",
        "regs",
        "usp",
        "ssp",
        "sr",
        "pc",
        "ir",
        "irc",
        "imask",
        "bus",
        "buslog",
        "dis",
        "words",
        "text",
        "trace",
        "stack",
        "mem",
        "hex",
        "base",
        "wlog",
        "console_delta",
        "bps",
        "rom",
        "kind",
        "addr",
        "data",
        "fc",
        "uds",
        "lds",
        "pins",
        "expected",
        "actual",
        "diff",
        "cycles",
        "roms",
        "files",
        "stem",
        "tests",
        "name",
        "size",
        "version",
        "message",
        "word",
    ] {
        assert!(
            h.contains(&format!(".{f}")),
            "field `{f}` not used by the page"
        );
    }
    // message types handled
    for ty in ["hello", "roms", "sst_files", "state", "sst_test", "error"] {
        assert!(
            h.contains(&format!("m.type === '{ty}'")),
            "type `{ty}` not handled"
        );
    }
    // commands sent
    for cmd in [
        "hello",
        "list_roms",
        "load",
        "upload",
        "run",
        "pause",
        "step",
        "step_clock",
        "reset",
        "speed",
        "bp_add",
        "bp_del",
        "view",
        "sst_list",
        "sst_load",
        "console_clear",
    ] {
        assert!(
            h.contains(&format!("cmd: '{cmd}'")) || h.contains(&format!("'{cmd}'")),
            "cmd `{cmd}` never sent"
        );
    }
    assert!(h.contains("/ws"));
}

#[test]
fn e13_06_fr_en() {
    let h = page();
    let (fr, en) = (keys(&h, "fr"), keys(&h, "en"));
    assert!(fr.len() > 20, "{} keys", fr.len());
    assert_eq!(fr, en, "the two languages must have the same keys");
    // every key used by the markup exists
    let mut rest = h.as_str();
    while let Some(i) = rest.find("data-i18n=\"") {
        rest = &rest[i + 11..];
        let k = &rest[..rest.find('"').unwrap()];
        assert!(fr.contains(k), "missing translation key `{k}`");
    }
}

#[test]
fn e13_06_no_cpu_logic() {
    let h = page();
    let s = script(&h);
    for bad in [
        "<<",
        ">>",
        "0x",
        "Math.imul",
        "toString(2)",
        "^",
        "|=",
        "&=",
        " & ",
        " | ",
    ] {
        assert!(
            !s.contains(bad),
            "`{bad}` in the script: the page must not compute CPU values"
        );
    }
    for m in [
        "moveq", "addq", "subq", "cmpa", "movea", "jsr", "rts", "dbra", "ROXL", "ASL",
    ] {
        assert!(!s.contains(m), "mnemonic `{m}` in the script");
    }
    // ASCII only, as the rest of the repository
    assert!(h.is_ascii());
}
