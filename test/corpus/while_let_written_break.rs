// A `while let` whose first `if` the program wrote breaks: the loop's test
// is its `Some`, `while ((word = words[i]) != null)`, and the `if` stays
// the program's with its comment, of a test of the value's truth too,
// `if (!word)` of `word.is_empty()`, which said `word != null` already and
// so was joined into the loop's, `while ((word = words[i]))` (#301, ADR
// 0702 amended); and of an `if` whose test mixes `&&` and `||`, whose
// `break` MIR makes the way out of a block, React's workLoop (rd215). The
// same tests in the same order: a missing item and a blank both end the
// loop, as in Rust, and `yields` runs only where it did.
use std::cell::Cell;

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

fn until_due(times: &[u32], now: u32, has_time: bool, yields: &dyn Fn() -> bool) -> usize {
    let mut i = 0;
    while let Some(&time) = times.get(i) {
        if time > now && (!has_time || yields()) {
            // This one hasn't expired, and we've reached the deadline.
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
    let calls = Cell::new(0);
    let yields = || {
        calls.set(calls.get() + 1);
        calls.get() % 2 == 0
    };
    for (times, now, has_time) in [
        (&[1, 5, 2, 9][..], 4, true),
        (&[1, 5, 2, 9], 4, false),
        (&[], 4, true),
        (&[1, 2], 4, true),
    ] {
        let due = until_due(times, now, has_time, &yields);
        println!("{due} {}", calls.get());
    }
}
