// A `take()` over a `Vec`'s items, stepped, then lent `&mut` and folded
// from its back: what it skips past its `n` dropped as one slice, front to
// back, as `vec::IntoIter`'s `advance_back_by` drops it. Lent as a `&mut
// dyn DoubleEndedIterator`, its `rfold` is std's default over `next_back`,
// as a `dyn`'s vtable holds no fold; lent as a sized `&mut`, std's `&mut
// I` folds by `Take`'s own `try_rfold`. Each by a `rev()`'s `fold`, an
// `rfold` and a `try_rfold`, and through generic code given the `&mut`
// (#1996, with std's iterator structs; #1986, #1991).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn taken() -> std::iter::Take<std::vec::IntoIter<D>> {
    let mut a = (1..=5).map(D).collect::<Vec<_>>().into_iter().take(3);
    println!("next {:?}", a.next().map(|x| x.0));
    a
}

fn digits(acc: u32, x: D) -> u32 {
    acc * 10 + x.0
}

fn folded_back<I: DoubleEndedIterator<Item = D>>(it: I) -> u32 {
    it.rfold(0, digits)
}

fn tried_back<I: DoubleEndedIterator<Item = D>>(mut it: I) -> Option<u32> {
    it.try_rfold(0, |acc, x| Some(digits(acc, x)))
}

fn main() {
    let mut a = taken();
    let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
    println!("dyn rev fold {}", b.rev().fold(0, digits));
    let mut a = taken();
    let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
    println!("dyn rfold {}", b.rfold(0, digits));
    let mut a = taken();
    let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
    println!("dyn try_rfold {:?}", tried_back(b));
    let mut a = taken();
    let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
    println!("dyn given {}", folded_back(b));
    let mut a = taken();
    println!("sized rev fold {}", (&mut a).rev().fold(0, digits));
    let mut a = taken();
    println!("sized rfold {}", (&mut a).rfold(0, digits));
    let mut a = taken();
    println!("sized try_rfold {:?}", tried_back(&mut a));
    let mut a = taken();
    println!("sized given {}", folded_back(&mut a));
}
