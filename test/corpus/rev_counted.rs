// rev() of skip(n) and take(n), made here, of an ExactSizeIterator: from
// its back, as std's Skip and Take step, a map's calls from the back too.
struct D(u8);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let t = |x: &i32| {
        println!("f{x}");
        x * 10
    };
    let v = vec![1, 2, 3, 4, 5, 6];
    println!("{:?}", v.iter().skip(2).rev().collect::<Vec<_>>());
    println!("{:?}", v.iter().take(4).rev().collect::<Vec<_>>());
    println!("{:?}", v.iter().map(t).skip(3).rev().collect::<Vec<_>>());
    println!("{:?}", v.iter().map(t).take(2).rev().next());
    println!("{:?}", (1..=10u8).skip(7).rev().collect::<Vec<_>>());
    println!("{:?}", (1..=10u16).take(20).rev().collect::<Vec<_>>());
    println!("{:?}", (5..5).skip(1).rev().next());
    println!("{:?}", v.iter().skip(1).take(3).rev().collect::<Vec<_>>());
    println!("end");
}
