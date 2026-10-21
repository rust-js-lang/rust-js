// An `enumerate`, a `take` and a `skip` cloned where they've stepped (ADR
// 0525): each made by a function of its source and the count it keeps, its
// clone made again at the count it's at.
fn main() {
    let mut numbered = "abcde".chars().enumerate();
    numbered.next();
    let copy = numbered.clone();
    println!("{} {:?}", numbered.count(), copy.collect::<Vec<_>>());

    let mut first = (10..20).take(4);
    first.next();
    let rest = first.clone();
    println!("{:?} {}", first.collect::<Vec<_>>(), rest.sum::<i32>());

    let tail = (1..6).skip(2);
    let fresh = tail.clone();
    let mut stepped = tail;
    stepped.next();
    let after = stepped.clone();
    println!(
        "{:?} {:?} {:?}",
        fresh.collect::<Vec<_>>(),
        stepped.collect::<Vec<_>>(),
        after.collect::<Vec<_>>()
    );
}
