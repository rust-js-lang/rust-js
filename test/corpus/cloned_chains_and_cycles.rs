// A `map`, a `filter`, a `chain` and a `zip` cloned, and `cycle()` of
// each, which clones it each round, from where it is, a `zip`'s or a
// `chain`'s other side given as a `Vec` or an array too (#181).
fn cycles() {
    let a = [1, 2];
    println!(
        "{:?}",
        a.iter()
            .map(|x| {
                println!("map {x}");
                x + 1
            })
            .cycle()
            .take(5)
            .collect::<Vec<_>>()
    );
    println!(
        "{:?}",
        (1..10).filter(|x| x % 3 == 0).cycle().take(5).collect::<Vec<_>>()
    );
    println!("{:?}", (0..0).map(|x: i32| x).cycle().next());
    let mut it = (1..4).map(|x| x * 2).cycle();
    it.next();
    println!("{:?}", it.take(4).collect::<Vec<_>>());
    println!(
        "{:?}",
        ["a", "b"]
            .iter()
            .map(|s| s.to_uppercase())
            .cycle()
            .take(3)
            .collect::<Vec<_>>()
    );
    println!(
        "{:?}",
        [Some(1), None].iter().map(|o| *o).cycle().take(4).collect::<Vec<_>>()
    );
}

fn clones() {
    let a = [1, 2];
    let b = [3];
    let c = a.iter().chain(b.iter());
    let d = c.clone();
    println!("{:?} {}", c.collect::<Vec<_>>(), d.count());
    println!("{:?}", a.iter().chain(b.iter()).cycle().take(7).collect::<Vec<_>>());
    let mut e = a.iter().chain(b.iter());
    e.next();
    e.next();
    let f = e.clone();
    println!("{:?} {:?}", e.collect::<Vec<_>>(), f.collect::<Vec<_>>());
    let z = a.iter().zip(b.iter().chain(a.iter()));
    println!("{:?} {:?}", z.clone().count(), z.collect::<Vec<_>>());
    println!("{:?}", (0..3).zip("ab".chars()).cycle().take(5).collect::<Vec<_>>());
    let m = a.iter().map(|x| x * 10);
    let m2 = m.clone();
    println!("{:?} {:?}", m.collect::<Vec<_>>(), m2.sum::<i32>());
    println!(
        "{:?}",
        (1..10).filter(|x| x % 3 == 0).cycle().take(5).collect::<Vec<_>>()
    );
    let words = vec!["x".to_string(), "y".to_string()];
}
fn more() {
    let v = vec![7, 8, 9];
    let mut z = (0..5).zip(10..13);
    z.next();
    let y = z.clone();
    println!("{:?} {:?}", z.collect::<Vec<_>>(), y.collect::<Vec<_>>());
    let mut c = [1, 2].iter().copied().chain([3, 4]);
    c.next();
    c.next();
    c.next();
    println!("{:?} {:?}", c.clone().collect::<Vec<_>>(), c.collect::<Vec<_>>());
    let mut d = v.iter().zip(&v);
    d.next();
    println!("{:?} {:?}", d.clone().collect::<Vec<_>>(), d.count());
    let cc = (0..2).chain(5..6).chain(9..10);
    let mut cc2 = cc.clone();
    cc2.next();
    println!("{:?} {:?}", cc.collect::<Vec<_>>(), cc2.collect::<Vec<_>>());
    let mut o = [1].into_iter().chain([3, 4].into_iter());
    o.next();
    o.next();
    println!("{:?} {:?}", o.clone().collect::<Vec<_>>(), o.collect::<Vec<_>>());
    let mut p = vec![1].into_iter().zip(vec![5, 6].into_iter().skip(1));
    let q = p.clone();
    println!("{:?} {:?}", p.next(), q.collect::<Vec<_>>());
}

fn stepped_then_cloned() {
    let mut m = vec![1, 2, 3].into_iter().map(|x| x * 10);
    m.next();
    let m2 = m.clone();
    println!("{:?} {:?}", m.collect::<Vec<_>>(), m2.collect::<Vec<_>>());
    let tail = vec![3, 4];
    let mut c = [1, 2].iter().chain(tail.iter());
    c.next();
    c.next();
    c.next();
    let d = c.clone();
    println!("{:?} {:?}", c.collect::<Vec<_>>(), d.collect::<Vec<_>>());
    let mut e = vec![1, 2].into_iter().chain(vec![3, 4].into_iter());
    e.next();
    e.next();
    e.next();
    let f = e.clone();
    println!("{:?} {:?}", e.collect::<Vec<_>>(), f.collect::<Vec<_>>());
    let mut z = vec![1, 2, 3].into_iter().zip(vec![4, 5, 6]);
    z.next();
    let y = z.clone();
    println!("{:?} {:?}", z.collect::<Vec<_>>(), y.collect::<Vec<_>>());
}

fn main() {
    cycles();
    clones();
    stepped_then_cloned();
}
