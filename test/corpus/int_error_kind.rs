// `ParseIntError::kind()`, and `{:?}` of another crate's enum without
// fields, std's `IntErrorKind`: its variant's name, as deranged's errors
// keep and show it.

use std::num::IntErrorKind;

#[derive(Debug, Clone, PartialEq)]
enum Bounded {
    Parse(IntErrorKind),
    OutOfRange { kind: IntErrorKind, limit: u8 },
}

fn read(text: &str, max: u8) -> Result<u8, Bounded> {
    let n = text.parse::<u8>().map_err(|e| Bounded::Parse(e.kind().clone()))?;
    if n > max {
        return Err(Bounded::OutOfRange { kind: IntErrorKind::PosOverflow, limit: max });
    }
    Ok(n)
}

fn main() {
    for text in ["7", "", "x1", "300", "-1", "90"] {
        println!("{:?}", read(text, 50));
    }
    let kind = "".parse::<i32>().unwrap_err().kind().clone();
    println!("{} {:?} {:#?}", kind == IntErrorKind::Empty, kind, IntErrorKind::Zero);
}
