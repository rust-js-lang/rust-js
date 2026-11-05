// A `skip(n)` or a `take(n)` of a `Vec`'s items with a destructor, stepped
// from its back, made in place or held in a variable first, alone or as a
// chain's side (#718): what it gives from its back dropped as each is
// taken, and what it skipped or left at its front dropped as the `Vec`'s
// `IntoIter` drops what it has left, front to back, as Rust drops it. What
// a `take`'s `rfold` trims and what an `nth_back` past either's end finds
// are dropped too, as std lets go of them; and of numbers, a held `skip`'s
// and `take`'s `rev()`, and the `len()` of a held one reversed. Each way is
// of items of a type of its own, `P` made in place, `C` a chain's sides and
// `D` held, a `rev()` of one held too, as what's made knowing its back is
// decided of each type.
struct P(u32);
struct C(u32);
struct D(u32);

impl Drop for P {
    fn drop(&mut self) {
        println!("drop p{}", self.0);
    }
}

impl Drop for C {
    fn drop(&mut self) {
        println!("drop c{}", self.0);
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop d{}", self.0);
    }
}

fn ds<T>(make: fn(u32) -> T) -> Vec<T> {
    (1..=5).map(make).collect()
}

fn in_place() {
    println!("{}", ds(P).into_iter().skip(1).rev().map(|d| d.0).sum::<u32>());
    println!("{}", ds(P).into_iter().take(3).rev().map(|d| d.0).sum::<u32>());
    println!("{:?}", ds(P).into_iter().take(3).rev().nth(5).map(|d| d.0));
    println!("{:?}", ds(P).into_iter().skip(2).rev().nth(5).map(|d| d.0));
}

fn chain_sides() {
    let w = vec![C(10), C(11)];
    println!("{}", ds(C).into_iter().skip(1).chain(w).rev().map(|d| d.0).sum::<u32>());
    let w = vec![C(20)];
    println!("{}", ds(C).into_iter().take(2).chain(w).rev().map(|d| d.0).sum::<u32>());
    let w = vec![C(30)];
    let taken = ds(C).into_iter().take(3).chain(w).rev().nth(2);
    println!("{:?}", taken.map(|d| d.0));
    let w = vec![C(40)];
    let c = ds(C).into_iter().skip(3).chain(w);
    println!("{:?}", c.rev().map(|d| d.0).collect::<Vec<_>>());
}

fn held() {
    let held = ds(D).into_iter().skip(1);
    println!("{}", held.rev().map(|d| d.0).sum::<u32>());
    let held = ds(D).into_iter().take(3);
    println!("{}", held.rev().map(|d| d.0).sum::<u32>());
    let mut it = ds(D).into_iter().skip(2);
    println!("{:?}", it.next_back().map(|d| d.0));
    println!("{:?}", it.next().map(|d| d.0));
    println!("{:?}", it.rfold(0, |a, d| a * 10 + d.0));
    let mut it = ds(D).into_iter().take(4);
    println!("{:?}", it.nth_back(1).map(|d| d.0));
    println!("{:?}", it.rfind(|d| d.0 == 1).map(|d| d.0));
    drop(it);
    let held = ds(D).into_iter().take(4);
    println!("{:?}", held.rev().nth(9).map(|d| d.0));
    let held = ds(D).into_iter().skip(1);
    println!("{}", held.rev().len());
    let mut r = ds(D).into_iter().take(3).rev();
    println!("{:?}", r.next().map(|d| d.0));
    println!("{:?}", r.next_back().map(|d| d.0));
    drop(r);
    let mut r = ds(D).into_iter().skip(1).rev();
    println!("{:?}", r.nth(1).map(|d| d.0));
    println!("{:?}", r.next_back().map(|d| d.0));
    drop(r);
    let v: Vec<u32> = vec![1, 2, 3, 4, 5];
    let held = v.iter().take(3);
    println!("{:?}", held.rev().collect::<Vec<_>>());
    let held = v.into_iter().skip(1);
    println!("{}", held.rev().sum::<u32>());
}

fn main() {
    in_place();
    chain_sides();
    held();
}
