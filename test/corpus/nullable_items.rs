// React's root's lane maps (ADR 0586, amended): a `#[rust_js::nullable]`
// field of a `Vec<Option<T>>` holds its items' `None` as `null`, written by
// index, taken, and given where its items are made, `createLaneMap(null)`.

use std::cell::RefCell;

struct FiberRoot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    transition_lanes: RefCell<Vec<Option<u32>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    hidden_updates: RefCell<Vec<Option<Vec<u32>>>>,
}

fn create_lane_map<T: Copy>(initial: T) -> Vec<T> {
    let mut lane_map = Vec::new();
    for _ in 0..4 {
        lane_map.push(initial);
    }
    lane_map
}

fn mark(root: &FiberRoot, index: usize, lane: u32) {
    let mut lanes = root.transition_lanes.borrow_mut();
    lanes[index] = Some(lanes[index].unwrap_or(0) | lane);
}

fn clear(root: &FiberRoot, index: usize) {
    if root.transition_lanes.borrow()[index].is_some() {
        root.transition_lanes.borrow_mut()[index] = None;
    }
}

fn hide(root: &FiberRoot, index: usize, update: u32) {
    let mut hidden = root.hidden_updates.borrow_mut();
    match &mut hidden[index] {
        Some(updates) => updates.push(update),
        None => hidden[index] = Some(vec![update]),
    }
}

fn take_hidden(root: &FiberRoot, index: usize) -> usize {
    let mut hidden = root.hidden_updates.borrow_mut();
    match hidden[index].take() {
        Some(updates) => updates.len(),
        None => 0,
    }
}

fn main() {
    let root = FiberRoot {
        transition_lanes: RefCell::new(create_lane_map(None)),
        hidden_updates: RefCell::new((0..4).map(|_| None).collect()),
    };
    mark(&root, 1, 4);
    mark(&root, 1, 1);
    mark(&root, 3, 2);
    println!("{:?}", root.transition_lanes.borrow());
    clear(&root, 1);
    clear(&root, 2);
    println!("{:?}", root.transition_lanes.borrow());
    hide(&root, 0, 7);
    hide(&root, 0, 8);
    hide(&root, 2, 9);
    println!("{} {} {}", take_hidden(&root, 0), take_hidden(&root, 0), take_hidden(&root, 2));
    println!("{:?}", root.hidden_updates.borrow());
}
