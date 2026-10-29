// A `&mut` to an item, or an item's field, given to a call whose later
// argument changes the index it was borrowed at: the place, fixed where
// it's borrowed, boxed and written back, not a handle on it taken for
// an object (#280).
fn add_to(place: &mut u32, n: u32) {
    *place += n;
}

fn bump(lanes: &mut Vec<u32>, mut index: usize) -> usize {
    add_to(&mut lanes[index], {
        index += 1;
        10
    });
    lanes.push(0);
    index
}

fn bump_plain(lanes: &mut [u32; 4], mut index: usize) -> usize {
    add_to(&mut lanes[index], {
        index += 1;
        20
    });
    index
}

struct P {
    x: u32,
}

fn bump_field(ps: &mut Vec<P>, mut i: usize) -> usize {
    add_to(&mut ps[i].x, {
        i += 1;
        5
    });
    i
}

fn main() {
    let mut v = vec![0, 0, 0];
    let next = bump(&mut v, 1);
    println!("{v:?} {next}");
    let mut a = [0u32; 4];
    let n = bump_plain(&mut a, 2);
    println!("{a:?} {n}");
    let mut ps = vec![P { x: 1 }, P { x: 2 }];
    let i = bump_field(&mut ps, 0);
    println!("{} {} {i}", ps[0].x, ps[1].x);
}
