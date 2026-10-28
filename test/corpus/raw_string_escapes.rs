// A literal's backslashes, as its source writes them: a raw string's are
// its text, `r"\u00C0"` six characters, as isAttributeNameSafe's pattern
// holds them for `new RegExp` to read, and a cooked string's `\\` one. Each
// way a literal reaches JS, a `const`, a local, a raw byte string,
// `concat!`'s pieces, a format string, a `match` of it and `include_str!`,
// keeps them, its length and its text as Rust's.

const START: &str = r"a\u00C0-\u00D6b";
const ESCAPED: &str = "a\\u00C0b";
const HASHED: &str = r#"a\"b\n\u{41}"#;
const BYTES: &[u8] = br"a\u00C0\x41";
const JOINED: &str = concat!(r"\u00C0", "\\n", '\\');
const READ: &str = include_str!("raw_string_escapes.txt");

fn kind(s: &str) -> u8 {
    match s {
        r"\u00C0" => 1,
        "\u{C0}" => 2,
        _ => 0,
    }
}

fn main() {
    for s in [START, ESCAPED, HASHED, JOINED, READ] {
        println!("{} {} {s:?} {s}", s.len(), s.chars().count());
    }
    println!("{} {BYTES:?}", BYTES.len());
    let local = r"\x41\t";
    println!("{} {local}", local.len());
    let formatted = format!(r"{}\u00C0{local}\n", 1);
    println!("{} {formatted}", formatted.len());
    println!(r"\u{{41}} {}", kind(r"\u00C0"));
    println!("{} {}", kind("\u{C0}"), kind("\\u{C0}"));
}
