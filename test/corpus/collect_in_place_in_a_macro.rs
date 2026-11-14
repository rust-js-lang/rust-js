// `collect()`s a macro writes, which std collects in place of the `Vec`
// each chain moves, its capacity the source's (#1259): each expansion's of
// its own source, as Rust's are (#1788); and a repetition's of `Vec`s
// made where they're moved, of no source alike. One repetition of sources
// apart is refused (#1789).
macro_rules! bumped {
    ($v:expr) => {
        $v.into_iter().map(|x: u32| x + 1).collect::<Vec<u32>>()
    };
}

macro_rules! each_bumped {
    ($($v:expr),*) => {
        vec![$($v.into_iter().map(|x: u32| x + 1).collect::<Vec<u32>>()),*]
    };
}

fn main() {
    let mut a = Vec::with_capacity(10);
    a.push(1u32);
    let mut b = Vec::with_capacity(3);
    b.push(2u32);
    let from_a = bumped!(a);
    let from_b = bumped!(b);
    println!("{} {} {:?}", from_a.len(), from_a.capacity(), from_a);
    println!("{} {} {:?}", from_b.len(), from_b.capacity(), from_b);
    for w in each_bumped!(vec![5u32, 6], vec![7u32]) {
        println!("{} {:?}", w.len(), w);
    }
}
