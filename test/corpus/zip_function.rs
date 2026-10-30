// std::iter::zip(a, b), a.into_iter().zip(b): each's items, an array's,
// a set's in order, a range's, an Option's.
use std::collections::BTreeSet;
use std::iter::zip;
fn main() {
    println!("{:?}", zip([1, 2], ['a', 'b']).collect::<Vec<_>>());
    let v = vec![10, 20, 30];
    let s: BTreeSet<char> = "zyx".chars().collect();
    println!("{:?}", zip(&v, &s).map(|(a, b)| format!("{a}{b}")).collect::<Vec<_>>());
    println!("{:?}", zip(1..4, v.iter().rev()).collect::<Vec<_>>());
    println!("{:?}", zip(Some(5), [1, 2]).collect::<Vec<_>>());
    // Either side's: the second's a range's and an Option's too.
    println!("{:?}", zip([1, 2, 3], 'a'..='c').collect::<Vec<_>>());
    println!("{:?}", zip([1, 2], Some(5)).collect::<Vec<_>>());
    let names = ["a", "b", "c"];
    for (i, n) in zip(0.., names) {
        println!("{i} {n}");
    }
    let total: i32 = zip(&v, [1, 2]).map(|(a, b)| a * b).sum();
    println!("{total}");
}
