// A `char`'s Unicode answers are Rust's own tables', not the JS engine's,
// whose Unicode version may be another (#1888, ADR 1163): U+3F5CD, which
// Unicode 18 assigns and 17 doesn't, is shown as itself and is alphabetic;
// `to_lowercase` ends a word's `Σ` as `ς`, past case-ignorable marks;
// `to_uppercase` maps one `char` to several, and those Unicode 18 added; whitespace is Rust's, U+0085
// in and U+FEFF out.

fn main() {
    let assigned = '\u{3f5cd}';
    println!("{assigned:?} {} {:?}", assigned.is_alphabetic(), "\u{3f5cd}\u{3134b}");
    for word in ["ΌΣΟΣ ΣΑ", "Σ", "ΑΣ", "ΑΣ\u{301}", "ΑΣ'Β", "ΑΣ.Β", "\u{301}Σ", "aΣb", "ǅΣ"] {
        println!("{word} {}", word.to_lowercase());
    }
    println!("{} {}", "straße ﬃ ŉ".to_uppercase(), "İ".to_lowercase().len());
    // Mappings Unicode 18 added, which a Unicode 17 engine leaves as they are.
    println!("{:?} {:?}", "\u{277}".to_uppercase(), "\u{a7dd}".to_lowercase());
    println!("{} {}", '\u{1246f}'.is_numeric(), '\u{558}'.is_lowercase());
    let upper: String = 'ﬃ'.to_uppercase().collect();
    let lower: Vec<char> = 'İ'.to_lowercase().collect();
    println!("{upper} {lower:?}");
    println!("{:?}", "a b\u{85}c\u{3000}d\u{feff}e".split_whitespace().collect::<Vec<_>>());
    println!("[{}] [{}] [{}]", "\u{85} x \u{feff}".trim(), " x ".trim_start(), " x ".trim_end());
    let asked: Vec<(bool, bool, bool)> = ['½', 'Ⅻ', '\u{1f600}', 'ǅ', 'ª']
        .iter()
        .map(|c| (c.is_numeric(), c.is_uppercase(), c.is_lowercase()))
        .collect();
    println!("{asked:?}");
    println!("{:?} {:?}", "\u{ff9e}\u{301}\u{200b}\u{e000}", "\u{a0}\u{2028}".escape_debug().to_string());
}
