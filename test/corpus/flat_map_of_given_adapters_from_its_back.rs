// A flatten stepped from its back whose items are adapters a function
// gives, std's structs of them (#1226): of a function given by its name,
// `flat_map(pairs)`, and of a closure that does more than return one,
// each made knowing its back as it's taken, as std's `FlatMap` steps them
// (#1235); `rev()`, `next_back`, `nth_back` and `rfold`, and what drops.
// And of a function computed where it's given, `flat_map(pick())`, which
// runs once, after its source, as Rust calls it: a `fn` pointer, an `impl
// Fn` and a `Box<dyn Fn>`; and of a closure that returns from its branches.

struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pairs(n: u32) -> impl DoubleEndedIterator<Item = u32> {
    (0..n).map(|x| x * 10)
}

fn nested() -> impl DoubleEndedIterator<Item = u32> {
    (1..4u32).flat_map(|x| pairs(x))
}

fn owned(items: Vec<D>) -> std::iter::Map<std::vec::IntoIter<D>, fn(D) -> u32> {
    items.into_iter().map(|d| d.0)
}

fn sized(n: u32) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator {
    (0..n).map(|x| x * 10)
}

fn named(n: u32) -> std::iter::Map<std::ops::Range<u32>, fn(u32) -> u32> {
    (0..n).map(|x| x + 1)
}

struct Doubled;

impl Doubled {
    fn of(n: u32) -> impl DoubleEndedIterator<Item = u32> {
        (0..n).map(|x| x * 2)
    }
}

// An associated function given by its path.
fn by_path() {
    println!("{:?}", (1..4u32).flat_map(Doubled::of).rev().collect::<Vec<_>>());
}

// Each other adapter of one a function gives, stepped from its back.
fn others() {
    println!("{:?}", sized(3).chain(sized(2)).rev().collect::<Vec<_>>());
    println!("{:?}", sized(3).zip(named(3)).rev().collect::<Vec<_>>());
    println!("{:?}", sized(4).enumerate().rev().collect::<Vec<_>>());
    println!("{:?}", sized(5).skip(1).rev().collect::<Vec<_>>());
    println!("{:?}", sized(5).take(3).rev().collect::<Vec<_>>());
    println!("{:?}", sized(5).filter(|x| x % 20 == 0).rev().collect::<Vec<_>>());
    println!("{:?}", sized(7).step_by(3).rev().collect::<Vec<_>>());
    println!("{:?}", named(4).map(|x| x * 2).rev().collect::<Vec<_>>());
    println!("{:?}", (1..3u32).map(sized).flatten().rev().collect::<Vec<_>>());
    println!("{:?}", [1u32, 2].iter().flat_map(|&n| named(n)).rev().collect::<Vec<_>>());
    println!("{:?}", sized(3).rev().skip(1).rev().collect::<Vec<_>>());
}

type Twice = std::iter::Map<std::array::IntoIter<u32, 2>, fn(u32) -> u32>;

fn source(n: u32) -> std::ops::Range<u32> {
    println!("source {n}");
    1..n
}

fn pick() -> fn(u32) -> Twice {
    println!("pick");
    fn each(n: u32) -> Twice {
        [n, n + 100].into_iter().map(|x| x * 2)
    }
    each
}

fn scaled(by: u32) -> impl Fn(u32) -> std::iter::Map<std::ops::Range<u32>, Box<dyn Fn(u32) -> u32>> {
    println!("scaled {by}");
    move |n| (0..n).map(Box::new(move |x| x * by) as Box<dyn Fn(u32) -> u32>)
}

fn boxed() -> Box<dyn Fn(u32) -> Twice> {
    println!("boxed");
    Box::new(|n| [n, n * 3].into_iter().map(|x| x + 1))
}

fn main() {
    let back: Vec<u32> = nested().rev().collect();
    println!("{back:?}");
    let mut it = nested();
    println!("{:?} {:?}", it.next_back(), it.next());
    println!("{:?}", nested().rfold(0, |a, x| a * 2 + x));
    println!("{:?}", nested().nth_back(2));
    let mut o = owned(vec![D(1), D(2), D(3)]);
    println!("{:?}", o.next_back());
    drop(o);
    println!("{:?}", pairs(3).rev().collect::<Vec<_>>());
    local();
    others();
    by_path();
}

fn local() {
    let v: Vec<u32> = (1..4u32).flat_map(pairs).rev().collect();
    println!("{v:?}");
    let w: Vec<u32> = (1..4u32).map(pairs).flatten().rev().collect();
    println!("{w:?}");
    let ds: Vec<u32> = (1..3u32).flat_map(|n| owned((0..n).map(D).collect())).rev().collect();
    println!("{ds:?}");
    let mut f = (1..4u32).flat_map(pairs);
    println!("{:?} {:?} {:?}", f.next(), f.next_back(), f.nth_back(1));
    let picked: Vec<u32> = source(4).flat_map(pick()).rev().collect();
    println!("{picked:?}");
    let mut p = source(3).flat_map(pick());
    println!("{:?} {:?} {:?}", p.next_back(), p.next(), p.nth_back(1));
    let last = source(4).flat_map(scaled(3)).next_back();
    println!("{last:?}");
    let folded = source(3).flat_map(boxed()).rfold(String::new(), |s, x| format!("{s}{x},"));
    println!("{folded}");
    let branched: Vec<u32> = (1..4u32)
        .flat_map(|n| if n > 1 { pairs(n) } else { pairs(5) })
        .rev()
        .collect();
    println!("{branched:?}");
    let matched: Vec<u32> = (1..4u32)
        .flat_map(|n| match n {
            1 => pairs(2),
            _ => {
                let m = n + 1;
                pairs(m)
            }
        })
        .rev()
        .collect();
    println!("{matched:?}");
}
