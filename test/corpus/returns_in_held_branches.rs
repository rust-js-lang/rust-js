// A `return` from a `match` inside an `if let` that holds an `Rc`, or a
// `Box`, whose drop MIR runs on each way out, is the `return` where it's
// written, as React's throwException returns from its switch on the
// Suspense handler's tag: no `let result` and labeled block the `return`s
// break out of (ADR 0432, amended).
use std::rc::Rc;

struct Boundary {
    tag: u8,
}

fn handler(tag: u8) -> Option<Rc<Boundary>> {
    (tag != 0).then(|| Rc::new(Boundary { tag }))
}

fn boxed(tag: u8) -> Option<Box<Boundary>> {
    handler(tag).map(|b| Box::new(Boundary { tag: b.tag }))
}

fn throw_exception(tag: u8, legacy: bool, log: &mut Vec<String>) -> bool {
    let boundary = handler(tag);
    if let Some(boundary) = boundary {
        match boundary.tag {
            1 | 2 => {
                if legacy {
                    log.push("shell".to_string());
                }
                return false;
            }
            4 => {
                if legacy {
                    return false;
                }
            }
            _ => {}
        }
        panic!("Unexpected tag ({})", boundary.tag);
    } else if legacy {
        return false;
    }
    log.push("error".to_string());
    true
}

fn of_a_box(tag: u8, legacy: bool) -> u8 {
    if let Some(boundary) = boxed(tag) {
        match boundary.tag {
            1 => return 10,
            4 if legacy => return 40,
            _ => {}
        }
        return boundary.tag;
    } else if legacy {
        return 1;
    }
    0
}

fn from_a_loop(tags: &[u8]) -> u8 {
    if let Some(boundary) = handler(tags.len() as u8) {
        for &tag in tags {
            if tag == boundary.tag {
                return tag * 2;
            }
        }
        panic!("no tag {}", boundary.tag);
    } else if tags.is_empty() {
        return 7;
    }
    9
}

fn main() {
    let mut log = Vec::new();
    println!("{}", throw_exception(1, true, &mut log));
    println!("{}", throw_exception(2, false, &mut log));
    println!("{}", throw_exception(4, true, &mut log));
    println!("{}", throw_exception(0, true, &mut log));
    println!("{}", throw_exception(0, false, &mut log));
    println!("{log:?}");
    let caught = std::panic::catch_unwind(|| throw_exception(4, false, &mut Vec::new()));
    println!("{}", caught.is_err());
    println!("{} {} {}", of_a_box(1, false), of_a_box(4, true), of_a_box(4, false));
    println!("{} {}", of_a_box(0, true), of_a_box(0, false));
    println!("{} {}", from_a_loop(&[3, 1, 2]), from_a_loop(&[]));
}
