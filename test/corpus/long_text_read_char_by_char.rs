// A long text read a `char` at a time, as a parser reads it (#1186): each
// step's `rest[i..].chars().next()`, `char_indices()` of what's left, and
// `next_back()` from its end cost what Rust's do, the `char`s they read.
// Each made an array of every `char` of what was left, `Array.from(s)`,
// so a loop over 100,000 of them didn't end in ten seconds.

// Each `char`'s kind, read one at a time from what's left.
fn count_digits(text: &str) -> (usize, usize) {
    let mut i = 0;
    let mut digits = 0;
    let mut chars = 0;
    while let Some(c) = text[i..].chars().next() {
        if c.is_ascii_digit() {
            digits += 1;
        }
        chars += 1;
        i += c.len_utf8();
    }
    (digits, chars)
}

// A tokenizer whose words are found by `char_indices()` of what's left.
fn words(text: &str) -> (usize, usize) {
    let mut pos = 0;
    let mut count = 0;
    let mut longest = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        let end = rest.char_indices().find(|(_, c)| *c == ' ').map_or(rest.len(), |(i, _)| i);
        if end > 0 {
            count += 1;
            longest = longest.max(end);
        }
        pos += end + rest[end..].chars().next().map_or(0, char::len_utf8);
    }
    (count, longest)
}

// Read from its end, a `char` at a time.
fn from_the_end(text: &str) -> (usize, char) {
    let mut end = text.len();
    let mut uppercase = 0;
    let mut first = ' ';
    while let Some(c) = text[..end].chars().next_back() {
        if c.is_uppercase() {
            uppercase += 1;
        }
        first = c;
        end -= c.len_utf8();
    }
    (uppercase, first)
}

// A `Chars` held and stepped from both ends, asked what's left.
fn both_ends(text: &str) -> (usize, String) {
    let mut chars = text.chars();
    let mut steps = 0;
    while chars.next().is_some() && chars.next_back().is_some() {
        steps += 1;
        if chars.as_str().len() < 8 {
            break;
        }
    }
    (steps, chars.as_str().to_string())
}

fn main() {
    let mut text = String::new();
    for i in 0..20_000 {
        text.push_str(match i % 4 {
            0 => "ab1 ",
            1 => "é2 ",
            2 => "Ω😀 ",
            _ => "Zz9y ",
        });
    }
    println!("{} chars, {} bytes", text.chars().count(), text.len());
    println!("digits {:?}", count_digits(&text));
    println!("words {:?}", words(&text));
    println!("from the end {:?}", from_the_end(&text));
    println!("both ends {:?}", both_ends(&text));
    let mut indices = text.char_indices();
    println!("{:?} {:?} {:?}", indices.nth(3), indices.next_back(), indices.size_hint());
}
