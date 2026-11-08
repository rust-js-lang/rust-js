// A map of items with a destructor given whole to generic code and back,
// std's compiled struct there, reversed: made knowing its back, `$knowing`
// of its compiled `next_back`, as `rev()` steps it from there, where `$rev`
// of it found no back (#999); of numbers too.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn hold<T>(t: T) -> T {
    t
}

fn main() {
    let r: Vec<u32> = hold(vec![Loud(4), Loud(5)].into_iter().map(|l| l)).rev().map(|l| l.0).collect();
    println!("{r:?}");
    let first = hold(vec![Loud(6), Loud(7), Loud(8)].into_iter().map(|l| l)).rev().next();
    println!("{:?}", first.map(|l| l.0));
    let n: Vec<i32> = hold(vec![4, 5].into_iter().map(|l| l)).rev().collect();
    println!("{n:?}");
}
