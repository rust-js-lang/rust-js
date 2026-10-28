// A `let` of a `match` whose other arm leaves, `let pairs = match e {
// Some(p) if .. => p, _ => { return; } };`, is a `let … else`'s guard,
// first, its comment with it, as React's trackDeletedPairViewTransitions
// writes `if (pairs === null || pairs.size === 0) { // We've found all.
// return; }`, the rest after it, not inside the guard negated; and the
// loop it ends with keeps its written `break` (ag6, ADR 0879, amended).
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type Pairs = Rc<RefCell<HashMap<String, u32>>>;

thread_local! {
    static PAIRS: RefCell<Option<Pairs>> = const { RefCell::new(None) };
    static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

struct Node {
    name: String,
    hidden: bool,
    child: RefCell<Option<Rc<Node>>>,
    sibling: RefCell<Option<Rc<Node>>>,
}

fn untrack(node: &Node, flags: u32) {
    let pairs = match PAIRS.with_borrow(Clone::clone) {
        Some(pairs) if !pairs.borrow().is_empty() => pairs,
        _ => {
            // We've found all.
            return;
        }
    };
    if flags == 0 {
        // This has no named view transitions in its subtree.
        return;
    }
    let mut child = node.child.borrow().clone();
    while let Some(current) = child {
        if current.hidden {
            // This tree was already hidden so we skip it.
        } else {
            let pair = pairs.borrow().get(&current.name).copied();
            if let Some(pair) = pair {
                pairs.borrow_mut().remove(&current.name);
                SEEN.with_borrow_mut(|seen| seen.push(pair));
                if pairs.borrow().is_empty() {
                    break;
                }
            }
            untrack(&current, flags);
        }
        child = current.sibling.borrow().clone();
    }
}

fn node(name: &str, hidden: bool, children: Vec<Rc<Node>>) -> Rc<Node> {
    let made = Rc::new(Node {
        name: name.to_string(),
        hidden,
        child: RefCell::new(children.first().cloned()),
        sibling: RefCell::new(None),
    });
    for pair in children.windows(2) {
        *pair[0].sibling.borrow_mut() = Some(Rc::clone(&pair[1]));
    }
    made
}

fn main() {
    let tree = node(
        "root",
        false,
        vec![
            node("a", false, vec![]),
            node("b", true, vec![]),
            node("c", false, vec![node("d", false, vec![])]),
            node("e", false, vec![]),
        ],
    );
    untrack(&tree, 1);
    let pairs: Pairs = Rc::new(RefCell::new(HashMap::from([
        ("a".to_string(), 1),
        ("b".to_string(), 2),
        ("d".to_string(), 4),
    ])));
    PAIRS.with_borrow_mut(|p| *p = Some(Rc::clone(&pairs)));
    untrack(&tree, 0);
    untrack(&tree, 1);
    println!("{:?} {}", SEEN.with_borrow(Clone::clone), pairs.borrow().len());
}
