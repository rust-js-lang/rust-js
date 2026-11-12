// A struct of a `NonZeroU32` matched against its constants, getrandom's
// `internal_desc`: rustc's MIR compares each by its number, a `transmute`
// of the pattern type `NonZero` holds (#1420).
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Error(NonZeroU32);

const UNSUPPORTED: Error = Error(NonZeroU32::new(1).unwrap());
const UNKNOWN: Error = Error(NonZeroU32::new(3).unwrap());

fn describe(error: Error) -> Option<&'static str> {
    match error {
        UNSUPPORTED => Some("unsupported"),
        UNKNOWN => Some("unknown"),
        _ => None,
    }
}

fn main() {
    for code in 1..5 {
        let error = Error(NonZeroU32::new(code).unwrap());
        println!("{code}: {:?} {:?}", describe(error), error.0.get());
    }
}
