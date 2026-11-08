// What the metamorphic net's `field` rewrite found, a value held in a
// tuple struct's field (#1035): `mem::swap(&mut spare.0, &mut pair.left)`
// of a `Vec` a tuple struct holds and one a field holds that's swapped
// whole elsewhere, a place each, where `$exchange` was given a handle; a
// tuple matched, `match (v.pop(), x.0)`, its `x.0` read where the tuple's
// made, before a guard sets it, directly, through a closure or through a
// `&mut` it's given; and a collect nothing reads, of an `inspect` chained,
// still collected, which steps what it spreads.
use std::mem;

struct Held<T>(T);

struct Pair {
    left: Vec<i32>,
    right: Vec<i32>,
}

fn bump(n: &mut u32) -> bool {
    *n += 10;
    false
}

fn guarded(v: &mut Vec<u32>) -> u32 {
    let mut x = Held(1);
    match (v.pop(), x.0) {
        (_, _)
            if {
                x.0 += 10;
                false
            } =>
        {
            0
        }
        (Some(b), a) => a * 100 + b + x.0,
        (None, a) => a + x.0,
    }
}

fn bumped(v: &mut Vec<u32>) -> u32 {
    let mut x = Held(1);
    match (v.pop(), x.0) {
        (_, _)
            if (|| {
                x.0 += 10;
                false
            })() =>
        {
            0
        }
        (Some(b), a) => a * 100 + b + x.0,
        (None, a) => a + x.0,
    }
}

fn bumped_by_ref(v: &mut Vec<u32>, first: bool) -> u32 {
    let mut x = Held(1);
    let mut y = Held(2);
    match (v.pop(), x.0) {
        (_, _) if bump(if first { &mut x.0 } else { &mut y.0 }) => 0,
        (Some(b), a) => a * 100 + b + x.0 + y.0,
        (None, a) => a + x.0 + y.0,
    }
}

fn probe(n: u32) -> u32 {
    println!("probe {n}");
    n
}

fn main() {
    let mut pair = Held(Pair {
        left: vec![1, 2],
        right: vec![3],
    });
    let old = mem::replace(&mut pair.0.left, vec![9]);
    let mut spare = Held(vec![7]);
    mem::swap(&mut spare.0, &mut pair.0.left);
    spare.0.push(8);
    mem::swap(&mut spare.0, &mut pair.0.left);
    println!("{:?} {:?} {:?} {:?}", old, pair.0.left, pair.0.right, spare.0);

    println!("{} {}", guarded(&mut vec![3]), guarded(&mut vec![]));
    println!("{} {}", bumped(&mut vec![4]), bumped(&mut vec![]));
    println!(
        "{} {}",
        bumped_by_ref(&mut vec![4], true),
        bumped_by_ref(&mut vec![], false)
    );

    for round in 0..probe(1) {
        let _again: Held<Vec<u32>> = Held(
            vec![probe(round), probe(round + 1)]
                .into_iter()
                .inspect(|seen| println!("see {seen}"))
                .chain(vec![probe(9)])
                .collect(),
        );
    }
    println!("end");
}
