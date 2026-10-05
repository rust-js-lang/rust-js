// Strings and chars ordered as Rust orders them, by code point, where a
// character past U+FFFF, 🦀, meets one from U+E000, a full-width `！` or
// U+FFFD, which JS's `<` puts the other way round; and `trim()` by
// Unicode's White_Space, U+0085 trimmed and U+FEFF kept (ADR 0183).

use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let crab = "🦀";
    let bang = "！";
    let words = vec!["b", bang, crab, "\u{fffd}", "a", "\u{e000}x", "🦀🦀"];
    println!("{} {} {:?} {:?}", crab < bang, crab > bang, crab.cmp(bang), crab.partial_cmp(bang));
    println!("{} {}", crab.max(bang), crab.min(bang));

    let mut sorted = words.clone();
    sorted.sort();
    println!("{sorted:?}");
    let mut by_key = words.clone();
    by_key.sort_by_key(|w| w.to_string());
    println!("{by_key:?}");
    println!("{:?} {:?}", words.iter().max(), words.iter().min());
    println!("{:?} {:?}", sorted.binary_search(&crab), sorted.binary_search(&"\u{fffd}"));

    let set: BTreeSet<&str> = words.iter().copied().collect();
    println!("{set:?}");
    let map: BTreeMap<String, usize> = words.iter().map(|w| (w.to_string(), w.len())).collect();
    println!("{map:?}");

    // Tuples and structs compare their strings so too.
    let mut pairs = vec![(crab, 1), (bang, 2), ("a", 3)];
    pairs.sort();
    println!("{pairs:?}");
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Name(String);
    println!("{}", Name(crab.to_string()) > Name(bang.to_string()));

    // Chars, and their ranges and patterns.
    let c = '🦀';
    println!("{} {:?} {}", c > '！', c.cmp(&'\u{fffd}'), ('\u{e000}'..='\u{ffff}').contains(&c));
    println!("{} {}", ('a'..='\u{ffff}').contains(&c), ('a'..'\u{fffd}').contains(&'！'));
    let private = matches!(c, '\u{e000}'..='\u{f8ff}');
    let ascii = matches!(c, 'a'..='z');
    let wide = matches!(c, 'a'..='\u{ffff}');
    println!("{private} {ascii} {wide} {}", matches!('q', 'a'..='z'));
    let mut chars: Vec<char> = "b！🦀\u{fffd}a".chars().collect();
    chars.sort();
    println!("{chars:?}");

    // Literal bounds below U+D800 compare as JS's `<` does.
    println!("{} {} {}", crab > "z", "m" < crab, "apple" < "banana");

    // trim() by White_Space.
    for s in ["\u{85}hi\u{85}", "\u{feff}hi\u{feff}", " \t hi \n", "\u{3000}hi\u{2028}"] {
        println!("{:?} {:?} {:?}", s.trim(), s.trim_start(), s.trim_end());
    }
}
