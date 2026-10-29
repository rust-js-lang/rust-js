// A `while let` whose first `if` the program wrote breaks: the loop's test
// is its `Some`, `while ((word = words[i]) != null)`, and the `if` stays
// the program's with its comment, of a test of the value's truth too,
// `if (!word)` of `word.is_empty()`, which said `word != null` already and
// so was joined into the loop's, `while ((word = words[i]))` (#301, ADR
// 0702 amended). The same tests in the same order: a missing item and a
// blank both end the loop, as in Rust.
fn until_blank(words: &[&str]) -> usize {
    let mut i = 0;
    while let Some(word) = words.get(i) {
        if word.is_empty() {
            // A blank ends it.
            break;
        }
        i += 1;
    }
    i
}

fn until_hash(words: &[&str]) -> usize {
    let mut i = 0;
    while let Some(word) = words.get(i) {
        if word.starts_with('#') {
            // A hash ends it.
            break;
        }
        i += 1;
    }
    i
}

fn main() {
    for words in [&["a", "b", "", "c"][..], &["a", "#b"], &[], &["", "a"], &["x", "y"]] {
        println!("{} {}", until_blank(words), until_hash(words));
    }
}
