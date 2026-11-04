// A `skip()` or a `take()` of a slice's items, `copied()`, stepped from its
// back by `rfold`, and a `filter()` between two `rev()`s of a `Vec`'s
// items with a `Drop`, held and stepped by `nth`: each made knowing its
// back where it's made, as JS's own `drop`, `take` and `filter` don't
// (#646).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let nums = [1u32, 2, 3, 4, 5];
    let e = nums.iter().skip(1).copied().rfold(0, |a, x| a * 10 + x);
    println!("{e}");
    let t = nums.iter().take(3).copied().rfold(0, |a, x| a * 10 + x);
    println!("{t}");
    let c = nums.iter().skip(1).copied().rev().fold(0, |a, x| a * 10 + x);
    println!("{c}");
    let v = vec![D(1), D(2), D(3), D(4), D(5)];
    let mut it = v.into_iter().rev().filter(|x| x.0 % 2 == 1).rev();
    let a = it.nth(1);
    println!("nth {:?}", a.as_ref().map(|d| d.0));
    drop(it);
    let w = vec![D(6), D(7), D(8), D(9), D(10)];
    let mut it = w.into_iter().rev().filter(|x| x.0 % 2 == 0).rev();
    println!("nth {:?}", it.nth(0).map(|d| d.0));
    println!("back {:?}", it.next_back().map(|d| d.0));
}
