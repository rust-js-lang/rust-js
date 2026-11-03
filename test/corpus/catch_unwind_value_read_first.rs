// An `Ok` arm that reads its value first, once, in what can't throw, React
// DOM's getActiveElement's nullable `Ok(active_element) => active_element`,
// whose `?? null` the result's conversion adds: the call where the value is
// read, `return doc.activeElement ?? doc.body ?? null;` (rd16). Not where
// the rest of the arm can throw, a panic Rust doesn't catch there, nor where
// the arm reads the value twice or after another read. As tidy reads any
// `const` read once where it's read, the program's name of it too.

use std::panic::{self, AssertUnwindSafe};

fn word_at(text: &str, at: usize) -> Option<String> {
    if text.is_empty() {
        panic!("no text");
    }
    text.split(' ').nth(at).map(str::to_string)
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn first_word(text: &str) -> Option<String> {
    match panic::catch_unwind(AssertUnwindSafe(|| word_at(text, 0))) {
        Ok(word) => word,
        Err(_) => None,
    }
}

// What the arm does after its value can throw: Rust's panic there isn't
// the `catch_unwind`'s.
fn expected_word(text: &str, at: usize) -> Option<String> {
    match panic::catch_unwind(AssertUnwindSafe(|| word_at(text, at))) {
        Ok(word) => Some(word.expect("a word there")),
        Err(_) => None,
    }
}

fn checked(limit: u32) -> u32 {
    if limit > 5 {
        panic!("too big");
    }
    limit
}

// Read first, by a sum whose other operand panics: that panic isn't the
// `catch_unwind`'s either.
fn summed(n: u32, limit: u32) -> u32 {
    match panic::catch_unwind(|| n * 2) {
        Ok(twice) => twice + checked(limit),
        Err(_) => 0,
    }
}

// Read twice: kept by its name.
#[cfg_attr(rust_js, rust_js::nullable)]
fn word_or_dash(text: &str) -> Option<String> {
    match panic::catch_unwind(AssertUnwindSafe(|| word_at(text, 1))) {
        Ok(word) => {
            if word.is_some() {
                word
            } else {
                Some("-".to_string())
            }
        }
        Err(_) => None,
    }
}

// Read after another read: kept by its name.
fn joined(text: &str, before: &str) -> String {
    match panic::catch_unwind(AssertUnwindSafe(|| word_at(text, 0))) {
        Ok(word) => before.to_string() + &word.unwrap_or_default(),
        Err(_) => String::new(),
    }
}

fn main() {
    println!("{:?} {:?}", first_word("hello world"), first_word(""));
    println!("{:?}", expected_word("hello world", 1));
    println!("{:?}", expected_word("", 1));
    let thrown = panic::catch_unwind(|| expected_word("hello", 3));
    println!("thrown past the arm: {}", thrown.is_err());
    println!("{}", summed(2, 3));
    let thrown = panic::catch_unwind(|| summed(2, 9));
    println!("thrown past the sum: {}", thrown.is_err());
    println!("{:?} {:?} {:?}", word_or_dash("a b"), word_or_dash("a"), word_or_dash(""));
    println!("{:?} {:?}", joined("a b", "x"), joined("", "x"));
}
