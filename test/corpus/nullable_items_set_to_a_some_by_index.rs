// An item of a nullable field's `Vec<Option<T>>` set by index to a
// `Some(..)` made there, React's addTransitionToLanesMap's
// `transition_lanes_map[index] = Some(transitions)`, is stored as it is,
// `lanes[index] = transitions`, a `Some`'s value never `undefined`; one set
// `None` is `null`, as the field's items are (#1933 step 4 (F)).
use std::cell::RefCell;
use std::rc::Rc;

struct Set {
    size: u32,
}

struct Root {
    #[cfg_attr(rust_js, rust_js::nullable)]
    lanes: RefCell<Vec<Option<Rc<Set>>>>,
}

fn add(root: &Root, index: usize, made: Rc<Set>) {
    let mut lanes = root.lanes.borrow_mut();
    let transitions = lanes[index].clone().unwrap_or(made);
    lanes[index] = Some(transitions);
}

fn put(root: &Root, index: usize, value: Option<Rc<Set>>) {
    root.lanes.borrow_mut()[index] = value;
}

fn clear(root: &Root, index: usize) {
    root.lanes.borrow_mut()[index] = None;
}

fn shown(root: &Root) -> Vec<Option<u32>> {
    root.lanes.borrow().iter().map(|lane| lane.as_ref().map(|set| set.size)).collect()
}

fn main() {
    let root = Root {
        lanes: RefCell::new(vec![None, None, None]),
    };
    add(&root, 1, Rc::new(Set { size: 2 }));
    add(&root, 1, Rc::new(Set { size: 5 }));
    add(&root, 2, Rc::new(Set { size: 7 }));
    println!("{:?}", shown(&root));
    clear(&root, 1);
    put(&root, 2, None);
    put(&root, 0, Some(Rc::new(Set { size: 9 })));
    println!("{:?} {}", shown(&root), root.lanes.borrow().iter().filter(|lane| lane.is_none()).count());
}
