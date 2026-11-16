// Clones of std's compiled adapters over a `copied()` of a `map` in
// `main`, beside one given to generic code that clones it as its caller
// says: `main`'s dictionary of the clones, named as std's `copiedClone`
// that `show`'s reads, is named apart from it, `copiedClone$1` (#1975).
// Not `rev`'s, whose clone beside `show` throws (#1976).
fn show<I: Iterator + Clone>(it: I)
where
    I::Item: std::fmt::Debug,
{
    let mut copy = it.clone();
    copy.next();
    println!("{:?} {:?}", copy.collect::<Vec<_>>(), it.collect::<Vec<_>>());
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    let a = items.iter().map(|x| x).copied().map(|x| x + 1);
    let mut a2 = a.clone();
    a2.next();
    println!("map {:?} {:?}", a2.collect::<Vec<_>>(), a.collect::<Vec<_>>());
    let b = items.iter().map(|x| x).copied().enumerate();
    let mut b2 = b.clone();
    b2.next();
    println!("enumerate {:?} {:?}", b2.collect::<Vec<_>>(), b.collect::<Vec<_>>());
    let c = items.iter().map(|x| x).copied().skip(1);
    let mut c2 = c.clone();
    c2.next();
    println!("skip {:?} {:?}", c2.collect::<Vec<_>>(), c.collect::<Vec<_>>());
    let d = items.iter().map(|x| x).copied().take(3);
    let mut d2 = d.clone();
    d2.next();
    println!("take {:?} {:?}", d2.collect::<Vec<_>>(), d.collect::<Vec<_>>());
    let f = items.iter().map(|x| x).copied().step_by(2);
    let mut f2 = f.clone();
    f2.next();
    println!("step_by {:?} {:?}", f2.collect::<Vec<_>>(), f.collect::<Vec<_>>());
    let h = items.iter().map(|x| x).copied().peekable();
    let mut h2 = h.clone();
    h2.next();
    println!("peekable {:?} {:?}", h2.collect::<Vec<_>>(), h.collect::<Vec<_>>());
    let j = items.iter().map(|x| x).copied().chain(8..10);
    let mut j2 = j.clone();
    j2.next();
    println!("chain {:?} {:?}", j2.collect::<Vec<_>>(), j.collect::<Vec<_>>());
    // And given to generic code, which clones it as its caller says.
    show(items.iter().map(|x| x).copied().map(|x| x * 10));
}
