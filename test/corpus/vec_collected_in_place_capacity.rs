// A `collect()` std makes in place, in its source `Vec`'s buffer: where
// both items align alike, its capacity is the source's bytes over its own
// items' size, `src_cap * src_size / dst_size`, shrunk to that where it's
// no whole number (#1259); of items aligned apart, an ordinary collect,
// as many as it holds, or room for a filter's hint. Its source's capacity
// as it is where the chain moves it, after it grew, of a variable, a field
// of one or a `Box`'s. Through generic code
// too, which its caller tells how std collects it (`IVecCollect`, #1239).
struct Held {
    items: Vec<u32>,
}

fn gather<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn main() {
    let mut same: Vec<u32> = Vec::with_capacity(10);
    same.extend([1, 2, 3]);
    let doubled: Vec<u32> = same.into_iter().map(|x| x * 2).collect();
    println!("{doubled:?} {}", doubled.capacity());

    let mut pairs: Vec<[u32; 2]> = Vec::with_capacity(3);
    pairs.extend([[1, 2], [3, 4]]);
    let firsts: Vec<u32> = pairs.into_iter().map(|[a, b]| a + b).collect();
    println!("{firsts:?} {}", firsts.capacity());

    let mut triples: Vec<[u32; 3]> = Vec::with_capacity(5);
    triples.extend([[1, 2, 3]]);
    let halves: Vec<[u32; 2]> = triples.into_iter().map(|[a, b, _]| [a, b]).collect();
    println!("{halves:?} {}", halves.capacity());

    let mut kept: Vec<u32> = Vec::with_capacity(7);
    kept.extend([1, 2, 3, 4]);
    let odd: Vec<u32> = kept.into_iter().filter(|x| x % 2 == 1).collect();
    println!("{odd:?} {}", odd.capacity());

    let mut wide: Vec<u64> = Vec::with_capacity(6);
    wide.extend([1, 2, 3, 4]);
    let narrow: Vec<u32> = wide.into_iter().map(|x| x as u32).collect();
    println!("{narrow:?} {}", narrow.capacity());

    let mut grown: Vec<u32> = Vec::with_capacity(2);
    grown.extend([1, 2, 3]);
    grown.reserve(6);
    let ones: Vec<u32> = grown.into_iter().map(|x| x % 2).collect();
    println!("{ones:?} {}", ones.capacity());

    let mut held = Held {
        items: Vec::with_capacity(6),
    };
    held.items.push(4);
    let from_field: Vec<u32> = held.items.into_iter().map(|x| x + 1).collect();
    println!("{from_field:?} {}", from_field.capacity());
    let mut boxed: Box<Vec<u32>> = Box::new(Vec::with_capacity(5));
    boxed.push(7);
    let from_box: Vec<u32> = (*boxed).into_iter().map(|x| x * 3).collect();
    println!("{from_box:?} {}", from_box.capacity());

    let mut given: Vec<u32> = Vec::with_capacity(9);
    given.extend([5, 6]);
    let gathered = gather(given.into_iter().map(|x| x + 1));
    println!("{gathered:?} {}", gathered.capacity());
    let mut plain: Vec<u32> = Vec::with_capacity(4);
    plain.push(8);
    let again = gather(plain.into_iter());
    println!("{again:?} {}", again.capacity());
}
