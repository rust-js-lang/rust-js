// A bit set's arithmetic, as React's lanes do it: each operator's
// assignment, a shift and a negation in a bitwise operator, at the edges
// where an `i32` wraps, `i32::MIN` and shifts by 31, 32 and 33.
use std::cell::Cell;

struct Root {
    pending_lanes: Cell<i32>,
    flags: i32,
    counts: [u32; 2],
}

fn removed(mut lanes: i32, lane: i32) -> i32 {
    lanes &= !lane;
    lanes |= 4;
    lanes ^= 2;
    lanes <<= 3;
    lanes >>= 1;
    lanes
}

fn halved(mut n: u32, by: u32) -> u32 {
    n >>= by;
    n <<= 1;
    n ^= 1;
    n
}

fn lowest(lanes: i32) -> i32 {
    lanes & -lanes
}

fn lane_of(index: u32) -> i32 {
    1 << index
}

fn equal_or_higher(lanes: i32) -> i32 {
    let index = 31 - (lanes as u32).leading_zeros();
    (1 << (index + 1)) - 1
}

fn masked_sum(a: i32, b: i32, mask: i32) -> i32 {
    (a + b) & mask
}

fn masked_sum_u32(a: u32, b: u32, mask: u32) -> u32 {
    (a + b) & mask
}

fn shifted_sum(a: i32, b: i32, by: u32) -> i32 {
    (a + b) << by
}

fn flipped_sum(a: i32, b: i32) -> i32 {
    !(a + b)
}

fn main() {
    let edges = [0, 1, 6, -1, i32::MIN, i32::MAX, 1 << 30];
    for lanes in edges {
        println!(
            "{lanes}: removed {} lowest {} negated {} higher {}",
            removed(lanes, 1),
            lowest(lanes),
            lanes.wrapping_neg(),
            equal_or_higher(lanes),
        );
    }
    for index in [0, 1, 30, 31, 32, 33] {
        println!("lane {index}: {}", lane_of(index));
    }
    for (n, by) in [(u32::MAX, 0), (u32::MAX, 31), (6, 32), (7, 33)] {
        println!("halved {n} {by}: {}", halved(n, by));
    }
    println!(
        "{} {} {} {}",
        masked_sum(i32::MAX, 1, -1),
        masked_sum(i32::MIN, -1, 0xff),
        masked_sum_u32(u32::MAX, 2, u32::MAX),
        masked_sum_u32(u32::MAX, u32::MAX, 0xf0),
    );
    println!(
        "{} {} {} {}",
        shifted_sum(i32::MAX, 1, 1),
        shifted_sum(1 << 30, 1 << 30, 0),
        flipped_sum(i32::MAX, 1),
        flipped_sum(i32::MIN, -1),
    );
    let mut root = Root {
        pending_lanes: Cell::new(1),
        flags: -1,
        counts: [3, 5],
    };
    for lane in [2, 8, i32::MIN] {
        root.pending_lanes.set(root.pending_lanes.get() | lane);
        root.flags &= !lane;
        root.counts[0] <<= 1;
        root.counts[1] ^= lane as u32;
    }
    println!("{} {} {:?}", root.pending_lanes.get(), root.flags, root.counts);
}
