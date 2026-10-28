// A loop's chain whose last arm is the `break` the program wrote, `} else
// if (owner.debugStack != null) { .. } else { break; }`, as React's
// getOwnerStackByComponentInfoInDev writes it, not `} else { if (..) {
// ..; continue; } break; }` of the `continue` MIR made at the body's end
// (ADR 0237, amended).
use std::cell::RefCell;
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

// A `while let`'s own test isn't a written `else`, whose rest would be
// its branch: React's propagateContextChanges' walk up to a sibling.
struct Node {
    id: u32,
    child: RefCell<Option<Rc<Node>>>,
    sibling: RefCell<Option<Rc<Node>>>,
    parent: RefCell<Option<Rc<Node>>>,
}

fn walk(root: &Rc<Node>, log: &mut Vec<u32>) {
    let mut fiber = root.child.borrow().clone();
    while let Some(current) = fiber {
        log.push(current.id);
        let mut next_fiber = current.child.borrow().clone();
        if let Some(next) = &next_fiber {
            next.parent.replace(Some(Rc::clone(&current)));
        } else {
            next_fiber = Some(current);
            while let Some(next) = next_fiber {
                if Rc::ptr_eq(&next, root) {
                    next_fiber = None;
                    break;
                }
                let sibling = next.sibling.borrow().clone();
                if let Some(sibling) = sibling {
                    let parent = next.parent.borrow().clone();
                    sibling.parent.replace(parent);
                    next_fiber = Some(sibling);
                    break;
                }
                next_fiber = next.parent.borrow().clone();
            }
        }
        fiber = next_fiber;
    }
}

fn node(id: u32, child: Option<Rc<Node>>, sibling: Option<Rc<Node>>) -> Rc<Node> {
    let node = Rc::new(Node {
        id,
        child: RefCell::new(child),
        sibling: RefCell::new(sibling),
        parent: RefCell::new(None),
    });
    if let Some(child) = node.child.borrow().as_ref() {
        child.parent.replace(Some(Rc::clone(&node)));
    }
    node
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
    let leaf = node(4, None, None);
    let tree = node(1, Some(node(2, Some(leaf), Some(node(3, None, None)))), None);
    let mut log = Vec::new();
    walk(&tree, &mut log);
    println!("{log:?}");
}
