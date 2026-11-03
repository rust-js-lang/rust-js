// An item written of what reads it, through a `RefCell`'s guard, by an
// index nothing changes meanwhile: written in place, `lanes[index] =
// (($index(lanes, index) ?? 0) | lane) >>> 0;`, no `const value` first, as
// React's markRootEntangled writes its lane maps.
use std::cell::RefCell;

fn mark(lanes: &RefCell<Vec<Option<u32>>>, index: usize, lane: u32) {
    let mut lanes = lanes.borrow_mut();
    lanes[index] = Some(lanes[index].unwrap_or(0) | lane);
}

fn main() {
    let lanes = RefCell::new(vec![None; 4]);
    mark(&lanes, 1, 4);
    mark(&lanes, 1, 1);
    mark(&lanes, 3, 2);
    println!("{:?}", lanes.borrow());
}
