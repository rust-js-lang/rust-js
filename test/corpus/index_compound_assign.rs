//@ run-fail: index out of bounds: the len is 4 but the index is 9
// An item's bitwise assignment, of a `RefCell`'s `Vec` at a `const` index
// or a parameter's, is its compound, its index checked: `v[$at(v, i)] |=
// lane`, and a sum's its wrap (ADR 0894, amended). Out of bounds, it
// panics before it reads or writes, as Rust's does.

use std::cell::RefCell;

struct Root {
    entanglements: RefCell<Vec<i32>>,
}

const SYNC_LANE_INDEX: usize = 1;

fn upgrade(root: &Root, lanes_to_upgrade: i32) {
    let mut lanes = lanes_to_upgrade;
    while lanes != 0 {
        let lane = lanes & -lanes;
        root.entanglements.borrow_mut()[SYNC_LANE_INDEX] |= lane;
        lanes &= !lane;
    }
}

fn spawned(root: &Root, index: usize, entangled: i32) {
    root.entanglements.borrow_mut()[index] |= 64 | (entangled & 8);
}

fn plain(v: &mut [i32], index: usize, lane: i32) {
    v[index] ^= lane;
    v[0] += lane;
}

fn main() {
    let root = Root {
        entanglements: RefCell::new(vec![0; 4]),
    };
    upgrade(&root, 0b1011);
    spawned(&root, 2, 0b1100);
    println!("{:?}", root.entanglements.borrow());
    let mut v = [1, 2, i32::MAX];
    plain(&mut v, 1, 3);
    plain(&mut v, 2, 1);
    println!("{v:?}");
    spawned(&root, 9, 1);
    println!("unreached");
}
