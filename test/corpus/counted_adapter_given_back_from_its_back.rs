// A `skip(n)`, a `take(n)` or a `step_by(n)` a function gives behind an
// `impl`, std's struct, stepped from its back by its caller, its `rev()`'s
// `len()` too; and a held one's, made knowing its back where it's made, its
// mark read past its maker, not its type (#814).
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn tail(v: Vec<D>) -> impl DoubleEndedIterator<Item = D> + ExactSizeIterator {
    v.into_iter().skip(1)
}
fn head(v: Vec<u32>) -> impl DoubleEndedIterator<Item = u32> + ExactSizeIterator {
    v.into_iter().take(3)
}
fn every(v: Vec<D>) -> impl DoubleEndedIterator<Item = D> {
    v.into_iter().step_by(2)
}
fn ds(n: u32) -> Vec<D> {
    (1..=n).map(D).collect()
}
fn main() {
    let r: Vec<u32> = tail(ds(4)).rev().map(|d| d.0).collect();
    println!("{r:?}");
    println!("{:?}", tail(ds(5)).nth_back(1).map(|d| d.0));
    println!("{:?}", head(vec![1, 2, 3, 4, 5]).rev().collect::<Vec<_>>());
    println!("{}", head(vec![1, 2, 3, 4, 5]).rev().len());
    let mut it = tail(ds(4));
    println!("{:?} {:?}", it.next_back().map(|d| d.0), it.next().map(|d| d.0));
    println!("{:?}", every(ds(7)).rev().map(|d| d.0).collect::<Vec<_>>());
    let held = ds(5).into_iter().skip(1);
    println!("{}", held.rev().len());
    let held = ds(5).into_iter().take(3);
    println!("{}", held.rev().len());
    println!("{}", tail(ds(5)).rev().len());
    println!("end");
}
