// React DOM's initInput, inside an `if` of a flag, a constant's `if` and
// an `Option`'s test: `if is_button || to_string(value) != node.value() {
// node.set_value(..) }` last in its block. Its `if` keeps its test as
// written, `if (isButton || toString(value) !== node.value) { .. }`, never
// an empty branch, `if (!isButton && ..) {} else { .. }` (rd503).
use std::cell::{Cell, RefCell};

const DISABLE_SYNCING: bool = true;

thread_local! {
    static CHANGE_EVENTS: Cell<bool> = const { Cell::new(false) };
}

struct Node {
    value: RefCell<String>,
    default_value: RefCell<String>,
}

impl Node {
    fn value(&self) -> String {
        self.value.borrow().clone()
    }

    fn set_value(&self, value: &str) {
        println!("set value {value:?}");
        *self.value.borrow_mut() = value.to_string();
    }
}

fn to_string(value: Option<u32>) -> String {
    value.map_or_else(String::new, |v| v.to_string())
}

fn init(node: &Node, value: Option<u32>, is_button: bool, is_hydrating: bool, initial_value: &str) {
    // Do not assign value if it is already set.
    if !is_hydrating || CHANGE_EVENTS.with(|on| on.get()) {
        if DISABLE_SYNCING {
            // Only assign it if it exists.
            if value.is_some() {
                // Always assign on buttons.
                if is_button || to_string(value) != node.value() {
                    node.set_value(&to_string(value));
                }
            }
        } else {
            // When syncing the value attribute.
            if initial_value != node.value() {
                node.set_value(initial_value);
            }
        }
    }
    if !DISABLE_SYNCING {
        *node.default_value.borrow_mut() = initial_value.to_string();
    }
}

fn main() {
    let node = Node {
        value: RefCell::new("3".to_string()),
        default_value: RefCell::new(String::new()),
    };
    init(&node, Some(3), false, false, "");
    init(&node, Some(4), false, false, "");
    init(&node, Some(4), true, false, "");
    init(&node, None, true, false, "");
    init(&node, Some(5), false, true, "");
    CHANGE_EVENTS.with(|on| on.set(true));
    init(&node, Some(6), false, true, "");
    println!("{} {:?}", node.value(), node.default_value.borrow());
}
