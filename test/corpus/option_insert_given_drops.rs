// `insert` and `get_or_insert` in generic code drop the value they don't
// keep by the drop they're given: a type parameter's, `dropT`, or an
// associated type's, `dropIIntoIter`, which is none where what it's called
// with has nothing to drop, so it's called only where it's there, as
// std's `flatten` does of its front and back iterators.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn put<T>(slot: &mut Option<T>, value: T) {
    slot.insert(value);
    println!("put");
}

fn kept<T>(slot: &mut Option<T>, value: T) {
    slot.get_or_insert(value);
    println!("kept");
}

fn put_iter<I: IntoIterator>(slot: &mut Option<I::IntoIter>, items: I) -> usize {
    slot.insert(items.into_iter()).count()
}

fn main() {
    let mut n = Some(1);
    put(&mut n, 2);
    kept(&mut n, 3);
    let mut d = Some(D(1));
    put(&mut d, D(2));
    kept(&mut d, D(3));
    println!("{n:?} {}", d.as_ref().map_or(0, |d| d.0));

    let mut owned = None;
    println!("{}", put_iter(&mut owned, vec![D(10), D(11)]));
    println!("{}", put_iter(&mut owned, vec![D(12)]));
    let v = vec![1, 2, 3];
    let mut lent = None;
    println!("{}", put_iter(&mut lent, &v));
    println!("{}", put_iter(&mut lent, &v[1..]));
    let mut numbers = None;
    println!("{}", put_iter(&mut numbers, vec![4, 5]));
    println!("{}", put_iter(&mut numbers, vec![6]));
}
