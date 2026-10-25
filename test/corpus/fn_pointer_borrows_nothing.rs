// A function pointer the crate makes only of functions that borrow no
// cell, React's `clz32` or its fallback, called while a `RefCell`'s borrow
// is held: no other borrow can meet it, and its cell is the array it
// holds (ADR 0904).

use std::cell::RefCell;

thread_local! {
    static CLZ32: fn(u32) -> u32 = chosen(true);
}

fn chosen(native: bool) -> fn(u32) -> u32 {
    if native { u32::leading_zeros } else { clz32_fallback }
}

fn clz32_fallback(x: u32) -> u32 {
    if x == 0 {
        return 32;
    }
    31 - x.ilog2()
}

struct Root {
    entanglements: RefCell<Vec<i32>>,
}

fn pick_arbitrary_lane_index(lanes: i32) -> usize {
    31 - CLZ32.with(|clz32| clz32(lanes as u32)) as usize
}

fn entangled_lanes(root: &Root, render_lanes: i32) -> i32 {
    let mut entangled_lanes = render_lanes;
    let entanglements = root.entanglements.borrow();
    let mut lanes = render_lanes;
    while lanes > 0 {
        let index = pick_arbitrary_lane_index(lanes);
        let lane = 1 << index;
        entangled_lanes |= entanglements[index];
        lanes &= !lane;
    }
    entangled_lanes
}

fn main() {
    let root = Root {
        entanglements: RefCell::new((0..31).map(|i| 1 << ((i + 3) % 31)).collect()),
    };
    for lanes in [0, 1, 6, 0b1010_0000, i32::MAX] {
        println!("{lanes} -> {}", entangled_lanes(&root, lanes));
    }
    let fallback = chosen(false);
    for x in [0, 1, 255, 1 << 20, u32::MAX] {
        println!("{x}: {} {}", fallback(x), x.leading_zeros());
    }
    root.entanglements.borrow_mut()[0] = -1;
    println!("{}", entangled_lanes(&root, 1));
}
