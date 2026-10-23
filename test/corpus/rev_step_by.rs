// rev() of a step_by(n) made in place, of an exact-sized source: its last
// kept item first, found by the source's length, as std's StepBy finds it.
fn main() {
    let t = |x: &i32| {
        println!("f{x}");
        *x
    };
    let v = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    println!("{:?}", (0..10).step_by(3).rev().collect::<Vec<_>>());
    println!("{:?}", (0..9).step_by(3).rev().collect::<Vec<_>>());
    println!("{:?}", v.iter().step_by(4).rev().collect::<Vec<_>>());
    println!("{:?}", v.iter().map(t).step_by(5).rev().collect::<Vec<_>>());
    println!("{:?}", (1..=10u8).step_by(2).rev().collect::<Vec<_>>());
    let mut s = v.iter().step_by(3);
    println!("{:?}", s.next());
    println!("{:?}", (0..0).step_by(2).rev().next());
    println!("{:?}", v.iter().skip(1).step_by(2).rev().take(2).collect::<Vec<_>>());
}
