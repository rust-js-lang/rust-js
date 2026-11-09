//@ library-differs
// A `RefCell` of a type only a caller knows, `I::IntoIter`, borrowed in
// the crate's `FromIterator` impl, which std's `collect` runs: what it
// borrows is a cell of some type there, not one of the caller's type
// parameters, which the caller's own args were given in its place, a
// panic of rustc's (#1017). What the metamorphic net's `rc-refcell`
// rewrite found of `held_adapters_stepped_to_their_end`.
use std::cell::RefCell;
use std::rc::Rc;

struct Sum(u32);

impl FromIterator<(u32, u32)> for Sum {
    fn from_iter<I: IntoIterator<Item = (u32, u32)>>(items: I) -> Self {
        let it = Rc::new(RefCell::new(items.into_iter()));
        let mut sum = 0;
        while let Some((a, b)) = it.borrow_mut().next() {
            sum += a * b;
        }
        Sum(sum)
    }
}

// And of a const parameter, `[u32; N]`, the impl's.
struct Fixed<const N: usize>([u32; N]);

impl<const N: usize> FromIterator<u32> for Fixed<N> {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let slots = Rc::new(RefCell::new([0; N]));
        for (at, item) in items.into_iter().take(N).enumerate() {
            slots.borrow_mut()[at] = item;
        }
        Fixed(*slots.borrow())
    }
}

fn main() {
    let Fixed(slots) = (5..9).collect::<Fixed<3>>();
    println!("{slots:?}");
    let Sum(sum) = (1..4).zip(10..).collect();
    println!("sum {sum}");
    let Sum(none) = std::iter::empty().collect();
    println!("none {none}");
}
