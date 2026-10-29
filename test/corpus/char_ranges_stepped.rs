// A `char` range stepped from both ends, and collected across the
// surrogates, which no `char` is: each step is a code point's.
fn main() {
    let mut r = 'a'..'e';
    let first = r.next();
    let last = r.next_back();
    let rest: Vec<char> = r.collect();
    let around: Vec<u32> = ('\u{D7FE}'..'\u{E001}').map(|c| c as u32).collect();
    let down: Vec<char> = ('p'..='s').rev().collect();
    let back: Vec<u32> = ('\u{D7FF}'..'\u{E001}').rev().map(|c| c as u32).collect();
    println!("{first:?} {last:?} {rest:?} {around:?} {down:?} {back:?}");
    // Reversed and stepped by, each step that many code points back, past
    // the surrogates too (#420).
    let every: String = ('a'..='e').rev().step_by(2).collect();
    let open: String = ('a'..'e').rev().step_by(3).collect();
    let across: Vec<u32> = ('\u{D7FD}'..='\u{E002}').rev().step_by(2).map(|c| c as u32).collect();
    println!("{every:?} {open:?} {across:?}");
}
