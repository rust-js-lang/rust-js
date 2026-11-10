//@ library-differs
// Calls nested 32 deep laid out in time linear in their depth: a number of
// 32 bits as a type, its dictionary nesting one call for each, as typenum's
// `U2147483648` does, and a call of a call 32 deep. oxc_formatter's
// `will_break` walked each interned element twice, so each level doubled
// the time, about 30 minutes at 32, past the corpus's compile limit (#1269).
use std::marker::PhantomData;

trait Number {
    fn value() -> u64;
}

struct Zero;

struct Twice<N>(PhantomData<N>);

struct TwicePlusOne<N>(PhantomData<N>);

impl Number for Zero {
    fn value() -> u64 {
        0
    }
}

impl<N: Number> Number for Twice<N> {
    fn value() -> u64 {
        N::value() * 2
    }
}

impl<N: Number> Number for TwicePlusOne<N> {
    fn value() -> u64 {
        N::value() * 2 + 1
    }
}

type Deep = TwicePlusOne<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<Twice<TwicePlusOne<Zero>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;

fn w(a: u64, b: u64) -> u64 {
    a.wrapping_mul(3).wrapping_add(b)
}

fn main() {
    println!("{}", <Deep as Number>::value());
    println!("{}", w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(w(1, 1), 2), 3), 4), 5), 6), 7), 8), 9), 10), 11), 12), 13), 14), 15), 16), 17), 18), 19), 20), 21), 22), 23), 24), 25), 26), 27), 28), 29), 30), 31), 32));
}
