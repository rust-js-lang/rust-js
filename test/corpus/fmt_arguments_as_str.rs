// `fmt::Arguments::as_str()` is `Some` of the text the arguments are (ADR
// 0034): its contract promises `Some` of a literal alone and allows `Some`
// of any, so anyhow's `format_err`, which keeps a literal as a `&'static
// str` and formats the rest, takes its first way for both.
use std::fmt;

enum Message {
    Literal(&'static str),
    Formatted(String),
}

impl Message {
    fn text(&self) -> &str {
        match self {
            Message::Literal(s) => s,
            Message::Formatted(s) => s,
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Literal(s) => write!(f, "literal {s}"),
            Message::Formatted(s) => write!(f, "formatted {s}"),
        }
    }
}

// anyhow's `format_err`.
fn message(args: fmt::Arguments<'_>) -> Message {
    match args.as_str() {
        Some(s) => Message::Literal(s),
        None => Message::Formatted(fmt::format(args)),
    }
}

fn main() {
    let count = 3;
    println!("{}", message(format_args!("plain")));
    // Which way a placeholder's takes, the contract leaves open: its text
    // is the same either way.
    println!("{}", message(format_args!("{count} items")).text());
    println!("{:?} {:?}", format_args!("").as_str(), format_args!("one").as_str());
}
