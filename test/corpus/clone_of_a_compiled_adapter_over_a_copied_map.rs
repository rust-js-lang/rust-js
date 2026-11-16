// A clone of std's compiled adapter, each held-flipped one, over a
// `copied()` of a `map` over a slice: the `map` under the `copied()` is
// made in place, a JS iterator its maker clones, so the adapter's clone
// clones its source as its maker does, never as std's `Map` its type
// names (#1973). Not `flatten`, whose clone over this is refused, nor
// `zip`, which std's build refuses to read by its index (#1974); nor one
// given to generic code that clones it beside these, which crashes (#1975).
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
    let e = items.iter().map(|x| x).copied().rev();
    let mut e2 = e.clone();
    e2.next();
    println!("rev {:?} {:?}", e2.collect::<Vec<_>>(), e.collect::<Vec<_>>());
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
}
