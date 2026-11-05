// An enumeration made knowing its back and its length, `$enumerate(src)`,
// read again past its maker: a reversed one's `len()`, an enumeration of a
// `rev()` reversed again, and a map's `enumerate().next_back()` of items
// with a destructor, held in a variable to drop them (#764).
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}
fn main() {
    println!("{}", (1..9u32).skip(2).enumerate().rev().len());
    let v: Vec<(usize, u32)> = vec![1u32, 2, 3, 4].into_iter().rev().enumerate().rev().collect();
    println!("{v:?}");
    println!("{:?}", ds().into_iter().map(|x| x.0 + 100).enumerate().next_back());
    let mut it = ds().into_iter().map(|x| x.0 * 2).enumerate();
    println!("{:?} {:?}", it.next_back(), it.next());
    println!("{:?} {}", it.next_back(), it.len());
    drop(it);
    println!("{}", vec![1u32, 2, 3].into_iter().rev().enumerate().rev().len());
    let e: Vec<(usize, u32)> = (0..6u32).skip(1).enumerate().rev().skip(1).collect();
    println!("{e:?}");
    println!(
        "{:?}",
        ds().into_iter()
            .rev()
            .enumerate()
            .rev()
            .map(|(i, d)| i as u32 * d.0)
            .sum::<u32>()
    );
}
