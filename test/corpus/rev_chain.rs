// rev() of a chain made in place: b's items from its back, then a's, as
// std's Chain steps them, a map's calls from the back too; an Option's
// into_iter(), [] or [x], is one side.
fn main() {
    let t = |x: i32| {
        println!("f{x}");
        x
    };
    let v = vec![1, 2, 3];
    let w = [7, 8];
    println!("{:?}", v.iter().chain(w.iter()).rev().collect::<Vec<_>>());
    println!("{:?}", (1..3).map(t).chain((10..12).map(t)).rev().take(3).collect::<Vec<_>>());
    println!("{:?}", v.iter().copied().chain(w).chain(20..22).rev().collect::<Vec<_>>());
    println!("{:?}", (0..2).chain(Some(9)).rev().collect::<Vec<_>>());
    println!("{:?}", "ab".chars().chain("cd".chars()).rev().collect::<String>());
}
