//@ library-differs
// A `&mut` to `by_ref()`'s lender, or to a closure, made a `&mut dyn` of a
// blanket impl's trait, `impl<T: ?Sized> GetMut<T> for T`, is the pair of
// the lender or the closure itself, as a type parameter given a `&mut` to
// an iterator or a function holds it (ADR 0061): generic code bounded by
// `DoubleEndedIterator` steps it from either end, and by `FnMut` calls it,
// through the `&mut T` the trait gives back (#1571). A `&mut` to a `Vec`
// is its box, as before (#1414).
trait GetMut<T: ?Sized> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T: ?Sized> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn back_of<I: DoubleEndedIterator<Item = u32>>(it: &mut dyn GetMut<I>) -> Option<u32> {
    it.get_mut().nth_back(0)
}

fn front_of<I: Iterator<Item = u32>>(it: &mut dyn GetMut<I>) -> Option<u32> {
    it.get_mut().next()
}

fn call_twice<F: FnMut() -> u32>(f: &mut dyn GetMut<F>) -> u32 {
    let first = (f.get_mut())();
    first + (f.get_mut())()
}

fn push_to(v: &mut dyn GetMut<Vec<u32>>) {
    v.get_mut().push(9);
}

fn main() {
    let mut items = vec![1u32, 2, 3].into_iter();
    println!("{:?}", back_of(&mut items.by_ref()));
    println!("{:?}", items.next());
    println!("{:?}", back_of(&mut items));
    println!("{:?}", items.next());

    let mut more = vec![4u32, 5, 6].into_iter();
    let mut lent = more.by_ref();
    println!("{:?}", front_of(&mut lent));
    println!("{:?}", lent.next());
    println!("{:?}", more.next());

    let mut count = 0;
    let mut counter = || {
        count += 1;
        count
    };
    println!("{}", call_twice(&mut counter));
    let mut by_ref = &mut counter;
    println!("{}", call_twice(&mut by_ref));
    println!("{count}");

    let mut v = vec![1u32];
    push_to(&mut v);
    println!("{v:?}");
}
