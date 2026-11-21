//@ library-differs
// A generic impl whose type holds what its parameter's `into_iter()` makes,
// `impl<A: IntoIterator> FromT<A> for One<A::IntoIter>`, takes `A`'s
// `IntoIterator` dictionary, made for each caller's `A`, as its method
// that gives it back does: its dictionary was one module const, its
// method given a JS iterable's whatever `A` was, so a caller's own
// iterator was stepped as an array of its fields (#2263).
struct Count(u8);

impl Iterator for Count {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.0 >= 9 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
}

trait FromT<T> {
    fn from_t(t: T) -> Self;
    fn name() -> &'static str;
}

struct One<T>(T);

impl<A: IntoIterator> FromT<A> for One<A::IntoIter> {
    fn from_t(t: A) -> Self {
        One(t.into_iter())
    }
    fn name() -> &'static str {
        "one"
    }
}

fn one<T, U>(u: U) -> One<T>
where
    One<T>: FromT<U>,
{
    println!("made by {}", One::<T>::name());
    One::from_t(u)
}

fn main() {
    let mut counted = one(Count(3));
    println!("count {:?} {:?}", counted.0.next(), counted.0.next());
    let rest: Vec<u8> = counted.0.collect();
    println!("rest {rest:?}");
    let mut items = one(vec![4u8, 5]);
    println!("vec {:?} {:?} {:?}", items.0.next(), items.0.next(), items.0.next());
    let mut direct = <One<_> as FromT<_>>::from_t(Count(7));
    println!("direct {:?}", direct.0.next());
}
