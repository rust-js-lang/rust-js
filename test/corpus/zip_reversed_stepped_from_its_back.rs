// A zip's `rev()` stepped from its back, by `nth_back` and by `rfold`,
// of a `Vec`'s items with a destructor: made knowing its back where it's
// made, `$rev($zipEnds(..))`, which steps both sides from their back as
// std's `Zip` of two `ExactSizeIterator`s does (#740, #718).
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let v = vec![1u32, 2, 3, 4];
    let w = vec![10u32, 20, 30];
    println!("{:?}", v.iter().zip(w.iter()).rev().nth_back(1));
    let a: Vec<D> = (1..4).map(D).collect();
    let b = vec![7u32, 8, 9];
    let s = a
        .into_iter()
        .zip(b)
        .rev()
        .rfold(String::new(), |acc, (d, x)| format!("{acc}{}:{x} ", d.0));
    println!("{s}");
}
