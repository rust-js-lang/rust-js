// A loop's chain whose last arm is the `break` the program wrote, `} else
// if (owner.debugStack != null) { .. } else { break; }`, as React's
// getOwnerStackByComponentInfoInDev writes it, not `} else { if (..) {
// ..; continue; } break; }` of the `continue` MIR made at the body's end
// (ADR 0237, amended).
use std::rc::Rc;

struct Owner {
    tag: Option<u32>,
    debug_stack: Option<String>,
    debug_owner: Option<Rc<Owner>>,
    owner: Option<Rc<Owner>>,
}

fn describe(start: &Rc<Owner>) -> String {
    let mut info = String::new();
    let mut owner = Some(Rc::clone(start));
    while let Some(current) = owner {
        if current.tag.is_some() {
            owner = current.debug_owner.clone();
            info += "f";
        } else if let Some(stack) = &current.debug_stack {
            owner = current.owner.clone();
            info += stack;
            if owner.is_some() {
                info += "\n";
            }
        } else {
            break;
        }
    }
    info
}

// The same of a `for`, its `else` of a `break` after an arm that leaves
// another way.
fn first_long(words: &[&str]) -> Option<usize> {
    let mut seen = 0;
    for word in words {
        if word.len() > 5 {
            return Some(seen);
        } else if word.len() > 1 {
            seen += word.len();
        } else {
            break;
        }
    }
    if seen > 3 { Some(seen) } else { None }
}

fn main() {
    let leaf = Rc::new(Owner {
        tag: None,
        debug_stack: Some("at leaf".to_string()),
        debug_owner: None,
        owner: None,
    });
    let stop = Rc::new(Owner {
        tag: None,
        debug_stack: None,
        debug_owner: None,
        owner: None,
    });
    let middle = Rc::new(Owner {
        tag: None,
        debug_stack: Some("at middle".to_string()),
        debug_owner: None,
        owner: Some(Rc::clone(&leaf)),
    });
    let top = Rc::new(Owner {
        tag: Some(1),
        debug_stack: None,
        debug_owner: Some(Rc::clone(&middle)),
        owner: None,
    });
    let stopped = Rc::new(Owner {
        tag: Some(2),
        debug_stack: None,
        debug_owner: Some(stop),
        owner: None,
    });
    println!("{:?} {:?}", describe(&top), describe(&stopped));
    println!(
        "{:?} {:?} {:?}",
        first_long(&["ab", "cd", "efghijk"]),
        first_long(&["ab", "c", "efghijk"]),
        first_long(&["abc", "de"])
    );
}
