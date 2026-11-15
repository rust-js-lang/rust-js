// A `collect()` std makes in place, in its source `Vec`'s buffer, through a
// function of the crate's that gives its parameter's iterator, as it is or
// adapted: its capacity is the source's, `src_cap * src_size / dst_size`,
// as where the chain is written in the body (#1934). The function's MIR
// says which parameter its result is made of (`returned_from`).
fn opaque<I: Iterator<Item = u32> + ExactSizeIterator>(it: I) -> impl Iterator<Item = u32> + ExactSizeIterator {
    it
}

fn doubled<I: Iterator<Item = u32>>(it: I) -> impl Iterator<Item = u32> {
    it.map(|x| x * 2)
}

fn evens(v: Vec<u32>) -> impl Iterator<Item = u32> {
    v.into_iter().filter(|x| x % 2 == 0)
}

fn second<I: Iterator<Item = u32>>(_first: Vec<u32>, it: I) -> impl Iterator<Item = u32> {
    it.enumerate().map(|(i, x)| x + i as u32)
}

fn main() {
    let mut kept = Vec::with_capacity(10);
    kept.extend([1, 2, 3]);
    let d: Vec<u32> = opaque(kept.into_iter()).map(|x| x * 2).collect();
    println!("{d:?} {}", d.capacity());

    let mut kept = Vec::with_capacity(7);
    kept.extend([4, 5]);
    let d: Vec<u32> = doubled(kept.into_iter()).collect();
    println!("{d:?} {}", d.capacity());

    let mut v = Vec::with_capacity(9);
    v.extend([1, 2, 3, 4]);
    let d: Vec<u32> = evens(v).map(|x| x / 2).collect();
    println!("{d:?} {}", d.capacity());

    let mut kept = Vec::with_capacity(6);
    kept.extend([1, 1, 1]);
    let d: Vec<u32> = second(vec![9], kept.into_iter()).collect();
    println!("{d:?} {}", d.capacity());
}
