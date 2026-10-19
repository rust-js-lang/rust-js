// What nested loops read as, as a person writes them: a `let` in the inner
// loop is its own, declared where it's made; the inner loop's `break`, which
// ends the outer loop's turn too, is its `break`; and heck's word splitter,
// a `while let` with a `peek()` and an `if`'s chain, reads as written.

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Boundary,
    Lowercase,
    Uppercase,
}

fn scores(words: &[&str]) -> u32 {
    let mut total = 0;
    for word in words {
        let mut last = 0;
        for c in word.chars() {
            let score = if c.is_uppercase() { 2 } else { last };
            last = score + 1;
        }
        total += last;
    }
    total
}

fn trailing(s: &str) -> String {
    let mut out = String::new();
    for word in s.split(' ') {
        let mut chars = word.chars().peekable();
        while let Some(c) = chars.next() {
            if let Some(&next) = chars.peek() {
                if next == 'x' {
                    out.push(c);
                } else {
                    out.push(next);
                }
            } else {
                out.push('.');
                break;
            }
        }
    }
    out
}

fn split_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    for word in s.split(|c: char| !c.is_alphanumeric()) {
        let mut char_indices = word.char_indices().peekable();
        let mut init = 0;
        let mut mode = Mode::Boundary;
        while let Some((i, c)) = char_indices.next() {
            if let Some(&(next_i, next)) = char_indices.peek() {
                let next_mode = if c.is_lowercase() {
                    Mode::Lowercase
                } else if c.is_uppercase() {
                    Mode::Uppercase
                } else {
                    mode
                };
                if next_mode == Mode::Lowercase && next.is_uppercase() {
                    words.push(word[init..next_i].to_string());
                    init = next_i;
                    mode = Mode::Boundary;
                } else if mode == Mode::Uppercase && c.is_uppercase() && next.is_lowercase() {
                    words.push(word[init..i].to_string());
                    init = i;
                    mode = Mode::Boundary;
                } else {
                    mode = next_mode;
                }
            } else {
                words.push(word[init..].to_string());
                break;
            }
        }
    }
    words
}

fn main() {
    println!("{}", scores(&["aB", "Cd", "xyz"]));
    println!("{}", trailing("abx cd e"));
    println!("{:?}", split_words("XMLHttpRequest camelCase snake_case"));
}
