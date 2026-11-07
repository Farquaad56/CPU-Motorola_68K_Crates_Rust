//! Contract of the WebAssembly demo page `demo/index.html`.

use std::collections::BTreeSet;

fn page() -> String {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/demo/index.html");
    std::fs::read_to_string(p).expect("demo/index.html")
}

/// Keys of the translation table of `lang`.
fn keys(html: &str, lang: &str) -> BTreeSet<String> {
    let open = format!("  {lang}: {{");
    let start = html.find(&open).unwrap_or_else(|| panic!("table {lang}")) + open.len();
    let end = html[start..].find("\n  }").expect("end of table") + start;
    html[start..end]
        .lines()
        .filter_map(|l| {
            let (k, v) = l.trim().split_once(':')?;
            (k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && v.trim_start().starts_with('"'))
            .then(|| k.to_string())
        })
        .collect()
}

#[test]
fn e14_02_title() {
    let h = page();
    assert!(h.contains("<title>Motorola 68000 Emulator</title>"));
}

#[test]
fn e14_02_loads_wasm() {
    let h = page();
    assert!(h.contains("from './pkg/puce_m68k_wasm.js'"));
    assert!(h.contains("await init()"));
    // methods of the module used by the page
    for f in [
        "regs_json",
        "disasm",
        "mem_hex",
        "buslog_json",
        "step_clock",
        "load",
        "reset",
        ".step()",
    ] {
        assert!(h.contains(f), "`{f}` not used");
    }
}

#[test]
fn e14_02_tailwind() {
    let h = page();
    assert!(h.contains("https://cdn.tailwindcss.com"));
}

#[test]
fn e14_02_fr_en() {
    let h = page();
    let (fr, en) = (keys(&h, "fr"), keys(&h, "en"));
    assert!(fr.len() > 15, "{} keys", fr.len());
    assert_eq!(fr, en);
    let mut rest = h.as_str();
    while let Some(i) = rest.find("data-i18n=\"") {
        rest = &rest[i + 11..];
        let k = &rest[..rest.find('"').unwrap()];
        assert!(fr.contains(k), "missing translation key `{k}`");
    }
    assert!(h.is_ascii());
}
