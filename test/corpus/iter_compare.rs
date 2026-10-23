// cmp, partial_cmp, lt, le, gt and ge of two iterators: each item in
// turn, as std's cmp_by steps them, the first Ordering that isn't Equal,
// or which ends first; partial_cmp's None where two items have none.
fn main() {
    let t = |x: i32| {
        println!("t{x}");
        x
    };
    println!("{:?} {:?}", [1, 2].iter().cmp([1, 3].iter()), (1..3).partial_cmp(1..2));
    println!("{:?}", (1..5).map(t).cmp([1, 2, 0, 9].into_iter()));
    println!("{:?} {:?}", [1.0, f64::NAN].iter().partial_cmp([1.0, 2.0].iter()), [0.5].iter().partial_cmp([0.5].iter()));
    println!("{} {} {} {}", (1..3).lt(1..4), [2].iter().le([2].iter()), "ab".chars().gt("b".chars()), [1.0, f64::NAN].iter().ge([1.0, 2.0].iter()));
    println!("{:?}", "abc".chars().cmp("abd".chars()));
    println!("{:?}", vec![String::from("a"), String::from("b")].iter().cmp(vec![String::from("a")].iter()));
    println!("{:?}", std::iter::empty::<u8>().cmp(std::iter::empty()));
    println!("{} {}", [1, 2].iter().lt([1, 2].iter()), (0..2).gt(0..2));
}
