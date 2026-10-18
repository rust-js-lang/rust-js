// `rev()` of a chain over a string's `char_indices()`, as writeable's KMP
// prefix walks its proper prefixes from the longest: the string's items,
// listed and reversed, each callback called from the back as Rust's
// `next_back` calls it (ADR 0164).
fn proper_prefixes(s: &str) -> Vec<usize> {
    s.char_indices().map(|(idx, _)| idx).rev().filter(|&idx| idx > 0).collect()
}

fn longest_border(needle: &str) -> usize {
    for k in needle.char_indices().map(|(idx, _)| idx).rev().filter(|&idx| idx > 0) {
        if needle.as_bytes().starts_with(&needle.as_bytes()[needle.len() - k..]) {
            return k;
        }
    }
    0
}

fn main() {
    println!("{:?}", proper_prefixes("héllo"));
    println!("{} {} {}", longest_border("abab"), longest_border("aaa"), longest_border("abc"));
    let mut seen = Vec::new();
    let backwards: Vec<char> = "xyz"
        .char_indices()
        .map(|(i, c)| {
            seen.push(i);
            c
        })
        .rev()
        .collect();
    println!("{backwards:?} {seen:?}");
}
