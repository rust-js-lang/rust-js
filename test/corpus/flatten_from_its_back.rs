// A flatten, and a flat_map of a function giving arrays, stepped from its back
// as std's are: its front and back inner lists each its own, the function
// run as each is reached (ADR 0545).
fn main() {
    let v = vec![vec![1, 2], vec![], vec![3, 4, 5], vec![6]];
    println!("{:?}", v.iter().flatten().rev().collect::<Vec<_>>());
    let mut it = v.iter().flatten();
    println!(
        "{:?} {:?} {:?} {:?}",
        it.next(),
        it.next_back(),
        it.next_back(),
        it.next()
    );
    println!("{:?}", it.collect::<Vec<_>>());
    let mut fm = (1..4).flat_map(|n| {
        println!("make {n}");
        vec![n; n as usize]
    });
    println!("{:?} {:?}", fm.next_back(), fm.next());
    println!("{:?}", fm.rev().collect::<Vec<_>>());
    let words = ["ab", "", "cde"];
    println!(
        "{:?}",
        words
            .iter()
            .map(|w| w.as_bytes())
            .flatten()
            .rev()
            .copied()
            .collect::<Vec<u8>>()
    );
    println!(
        "{:?} {:?}",
        v.iter().flatten().rfind(|&&x| x % 2 == 1),
        v.iter().flatten().nth_back(2)
    );
    println!("{:?}", v.iter().flatten().last());
}
