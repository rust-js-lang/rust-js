// A flatten of `Option`s, through a reference too, and a `flat_map` whose
// function returns one, stepped from its back: `rfold`, `rfind`,
// `next_back` and `rev()`, of items with a destructor too. Made in place,
// a filter of the `Some`s as a `filter_map` is, it's made knowing its
// back, `$filterEnds`, as std's `Flatten` steps an `Option`'s from either
// end, where it threw a `TypeError` as its back was asked (#1326); one held
// through a variable is std's struct.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn halves(x: &u32) -> Option<u32> {
    x.is_multiple_of(2).then_some(x / 2)
}

fn main() {
    let v = vec![Some(D(1)), None, Some(D(2)), Some(D(3))];
    let total = v.into_iter().flatten().rfold(0, |acc, d| acc + d.0);
    println!("rfold {total}");
    let w = [1u32, 2, 3, 4];
    println!("{:?}", w.iter().flat_map(halves).rfind(|x| *x < 2));
    println!("{}", w.iter().flat_map(halves).rfold(0, |a, x| a * 10 + x));
    let o = [Some(1u32), None, Some(3)];
    println!("{:?}", o.iter().flatten().rev().collect::<Vec<_>>());
    println!("{:?}", o.iter().flatten().next_back());
    println!("{:?}", o.iter().flatten().rfind(|x| **x > 1));
    let nested = [Some(Some(1u32)), Some(None), None];
    let unwrapped = nested.into_iter().flatten().rfold(Vec::new(), |mut a, x| {
        a.push(x);
        a
    });
    println!("{unwrapped:?}");
    let mut it = w.iter().flat_map(|x| (*x != 2).then_some(*x));
    println!("{:?} {:?} {:?}", it.next_back(), it.next(), it.next_back());
}
