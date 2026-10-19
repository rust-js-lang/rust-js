// A writer given to another as a callable: a function or a closure that
// writes to the `Formatter` it's given, called where it writes, as heck's
// case conversions are (ADR 0054).
use std::fmt;

fn lowercase(s: &str, f: &mut fmt::Formatter) -> fmt::Result {
    for c in s.chars() {
        write!(f, "{}", c.to_lowercase())?;
    }
    Ok(())
}

fn capitalize(s: &str, f: &mut fmt::Formatter) -> fmt::Result {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        write!(f, "{}", first.to_uppercase())?;
        lowercase(chars.as_str(), f)?;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum WordMode {
    Boundary,
    Lowercase,
    Uppercase,
}

// heck's: each word, where the case changes too, given to `with_word`,
// and `boundary` between them.
fn transform<F, G>(s: &str, mut with_word: F, mut boundary: G, f: &mut fmt::Formatter) -> fmt::Result
where
    F: FnMut(&str, &mut fmt::Formatter) -> fmt::Result,
    G: FnMut(&mut fmt::Formatter) -> fmt::Result,
{
    let mut first_word = true;
    for word in s.split(|c: char| !c.is_alphanumeric()) {
        let mut char_indices = word.char_indices().peekable();
        let mut init = 0;
        let mut mode = WordMode::Boundary;
        while let Some((i, c)) = char_indices.next() {
            if let Some(&(next_i, next)) = char_indices.peek() {
                let next_mode = if c.is_lowercase() {
                    WordMode::Lowercase
                } else if c.is_uppercase() {
                    WordMode::Uppercase
                } else {
                    mode
                };
                if next_mode == WordMode::Lowercase && next.is_uppercase() {
                    if !first_word {
                        boundary(f)?;
                    }
                    with_word(&word[init..next_i], f)?;
                    first_word = false;
                    init = next_i;
                    mode = WordMode::Boundary;
                } else if mode == WordMode::Uppercase && c.is_uppercase() && next.is_lowercase() {
                    if !first_word {
                        boundary(f)?;
                    } else {
                        first_word = false;
                    }
                    with_word(&word[init..i], f)?;
                    init = i;
                    mode = WordMode::Boundary;
                } else {
                    mode = next_mode;
                }
            } else {
                if !first_word {
                    boundary(f)?;
                } else {
                    first_word = false;
                }
                with_word(&word[init..], f)?;
                break;
            }
        }
    }
    Ok(())
}

struct Kebab<'a>(&'a str);

impl fmt::Display for Kebab<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0, lowercase, |f| write!(f, "-"), f)
    }
}

struct Title<'a>(&'a str);

impl fmt::Display for Title<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0, capitalize, |f| f.write_str(" "), f)
    }
}

// A closure that captures what it writes, and one that writes more than once.
struct Numbered<'a>(&'a str, u32);

impl fmt::Display for Numbered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut n = self.1;
        let numbered = |word: &str, f: &mut fmt::Formatter| {
            n += 1;
            write!(f, "{n}.")?;
            write!(f, "{word}")
        };
        transform(self.0, numbered, |f| write!(f, ", "), f)
    }
}

// A closure that pads to the width its `Formatter` was given.
struct Columns<'a>(&'a str);

impl fmt::Display for Columns<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0, |word, f| f.pad(word), |f| f.write_str("|"), f)
    }
}

fn main() {
    println!("{}", Kebab("Hello Big WORLD XMLHttpRequest"));
    println!("{}", Title("the ßtraße of heck"));
    println!("{}", Numbered("one two three", 4));
    println!("{:>4} {:<3}.", Columns("a bc"), Columns("x y"));
    // A case mapping shows the `char`s it hasn't given.
    let mut upper = 'ß'.to_uppercase();
    upper.next();
    println!("[{}] [{}]", 'ǆ'.to_lowercase(), upper);
}
