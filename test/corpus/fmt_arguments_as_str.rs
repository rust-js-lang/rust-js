// `fmt::Arguments::as_str()` is `Some` of a literal's text and `None` of
// arguments made with a value to format, as native Rust's are (ADR 0034,
// amended; #1091), so anyhow's `format_err`, which keeps a literal as a
// `&'static str` and formats the rest, takes the way native Rust's does:
// each way prints its own.
use std::fmt;

enum Message {
    Literal(&'static str),
    Formatted(String),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Literal(s) => write!(f, "literal {s}"),
            Message::Formatted(s) => write!(f, "formatted {s}"),
        }
    }
}

// anyhow's `format_err`, given what `as_str()` said where the arguments
// were made.
fn message(text: Option<&'static str>, args: fmt::Arguments<'_>) -> Message {
    match text {
        Some(s) => Message::Literal(s),
        None => Message::Formatted(fmt::format(args)),
    }
}

fn main() {
    let count = 3;
    let plain = format_args!("plain");
    let items = format_args!("{count} items");
    // A constant argument rustc writes into the literal.
    let constant = format_args!("{} items", 3);
    println!("{}", message(plain.as_str(), plain));
    println!("{}", message(items.as_str(), items));
    println!("{}", message(constant.as_str(), constant));
    println!(
        "{:?} {:?}",
        format_args!("{count} items").as_str(),
        format_args!("plain").as_str()
    );
    println!("{:?} {:?}", format_args!("").as_str(), format_args!("one").as_str());
}
