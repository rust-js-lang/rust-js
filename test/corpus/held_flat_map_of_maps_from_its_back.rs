// A `flat_map()` held in a variable, std's compiled `FlatMap`, whose
// closure gives a `map()` of each group: what the closure gives is std's
// struct too, as the `FlatMap` steps it, from its back as well, not a JS
// `.map()` chain made in place, which has no back, and threw (#726).
fn main() {
    let groups = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let mut it = groups.iter().flat_map(|g| g.iter().map(|x| x * 10));
    println!("{:?}", it.next_back());
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.next());
    println!("{:?} {:?}", it.next_back(), it.next());
    let held = groups.iter().flat_map(|g| g.iter().map(|x| x + 1));
    let back: Vec<i32> = held.rev().collect();
    println!("{back:?}");
    let mut enumerated = groups
        .iter()
        .flat_map(|g| g.iter().enumerate().map(|(i, x)| i as i32 * 100 + x));
    println!("{:?} {:?}", enumerated.next_back(), enumerated.next());
    let mut counted = groups.iter().flat_map(|g| g.iter().map(|x| x * 2));
    counted.next_back();
    println!("{}", counted.count());
}
