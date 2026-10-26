// A function that chains what it's given, any IntoIterators, behind an
// `impl Iterator`: std's `Chain`, which holds what each makes, so each is
// given its dictionary (ADR 0920); stepped from its back by `rfind`.
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Countdown {
    left: u32,
    _tag: Loud,
}
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.left > 0).then(|| {
            self.left -= 1;
            self.left
        })
    }
}
fn louds(n: u32) -> Vec<u32> {
    (0..n).collect()
}
fn chain<I: IntoIterator<Item = u32>, J: IntoIterator<Item = u32>>(i: I, j: J) -> impl Iterator<Item = u32> {
    i.into_iter().chain(j)
}
fn main() {
    let counted: Vec<u32> = chain(Countdown { left: 2, _tag: Loud(9) }, louds(2)).collect();
    println!("{counted:?}");
    let mut held = vec![1u32, 2, 3].into_iter().chain(4..6);
    let hint = held.size_hint();
    println!("{hint:?} {:?} {:?}", held.rfind(|x| x % 2 == 0), held.nth_back(1));
}
