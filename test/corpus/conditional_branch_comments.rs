// A comment above an `else if`'s value, as React DOM's
// compareDocumentPositionForEmptyFragment writes `// otherNode has been
// portaled into another container`, is above it: in its branch, where the
// chain stays `else if`, as upstream has it, and before the branch where
// it's a conditional, `? // ..\n $f64ToInt(..)`. A branch's value that's a
// call whose argument is a constant written there starts where the
// argument does, and the comment is the branch's, not the argument's, nor
// lost (rd617).
const DOCUMENT_POSITION_CONTAINED_BY: f64 = 16.0;
const DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: f64 = 32.0;

fn position(first_or_last: bool, disconnected: bool, first_result: u32) -> u32 {
    let result = if first_or_last {
        DOCUMENT_POSITION_CONTAINED_BY as u32
    } else if disconnected {
        // otherNode has been portaled into another container
        DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC as u32
    } else {
        first_result
    };
    if result & DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC as u32 != 0 {
        return result | 1;
    }
    result
}

fn position_of_any(a: bool, b: bool, c: bool, disconnected: bool, first: u32) -> u32 {
    let result = if a || b || c {
        DOCUMENT_POSITION_CONTAINED_BY as u32
    } else if disconnected {
        // otherNode has been portaled into another container
        DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC as u32
    } else {
        first
    };
    result + 1
}

fn main() {
    for (a, b, r) in [(true, false, 2), (false, true, 2), (false, false, 4)] {
        println!("{} {}", position(a, b, r), position_of_any(false, false, a, b, r));
    }
}
