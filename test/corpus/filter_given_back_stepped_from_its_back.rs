// A filter a function or a closure makes and gives back, stepped from its
// back by its caller: made knowing its back by its maker, by its value's
// fact, the caller's local's carried to the callee's return place (#851);
// beside one of its type only stepped forward, JS's own `filter`.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn odds(n: u32) -> std::iter::Filter<std::vec::IntoIter<D>, fn(&D) -> bool> {
    fn odd(d: &D) -> bool {
        d.0 % 2 == 1
    }
    (0..n).map(D).collect::<Vec<_>>().into_iter().filter(odd as fn(&D) -> bool)
}

fn main() {
    let last = odds(5).next_back().map(|d| d.0);
    println!("last {last:?}");
    println!("count {}", odds(5).count());
    let make = || (0..6u32).filter(|x| x % 3 == 0);
    let found = make().rfind(|x| *x > 0);
    println!("rfind {found:?} then {}", make().count());
    let held = || (0..5).map(D).collect::<Vec<_>>().into_iter().filter(|d| d.0 % 2 == 1);
    let folded = held().rfold(0, |a, d| a * 10 + d.0);
    println!("rfold {folded} then {:?}", held().last().map(|d| d.0));
}
