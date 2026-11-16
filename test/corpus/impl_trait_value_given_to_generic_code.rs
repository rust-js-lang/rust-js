// A trait method's `impl Trait` value given by value to generic code that
// drops it: given its drop, the impl's of what it hides, as a named
// associated type's value is (#2006). Each alone, in a tuple, a field, an
// `Option` and a `Vec`, and through generic code that gives it on; and of
// an impl whose values drop nothing; and given through a bound of its own,
// whose evidence names it by its method's word.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Source {
    type Named;
    fn named(&self, tag: &'static str) -> Self::Named;
    fn hidden(&self, tag: &'static str) -> impl Sized;
}

struct Louds;

impl Source for Louds {
    type Named = Loud;

    fn named(&self, tag: &'static str) -> Loud {
        Loud(tag)
    }

    fn hidden(&self, tag: &'static str) -> impl Sized {
        Loud(tag)
    }
}

struct Quiet;

impl Source for Quiet {
    type Named = u8;

    fn named(&self, _: &'static str) -> u8 {
        1
    }

    fn hidden(&self, tag: &'static str) -> impl Sized {
        tag
    }
}

struct Wrap<X>(X);

fn at_2<T>(_x: T) {}

fn relay<U>(x: U) {
    at_2(x);
    println!("relayed");
}

/// What generic code is given as through a bound of its own, named by the
/// value's type, `THidden` of `<T as Source>`'s `impl Trait`.
trait Same<X> {
    fn same(self) -> X;
}

impl<X> Same<X> for X {
    fn same(self) -> X {
        self
    }
}

fn relay_same<U>(x: impl Same<U>) {
    at_2(x.same());
}

fn paths<T: Source>(t: &T) {
    at_2(t.named("named alone"));
    at_2((t.named("named in a tuple"), 1u8));
    at_2(Wrap(t.named("named in a field")));
    at_2(Some(t.named("named in an option")));
    at_2(vec![t.named("named in a vec")]);
    at_2(t.hidden("hidden alone"));
    at_2((t.hidden("hidden in a tuple"), 1u8));
    at_2(Wrap(t.hidden("hidden in a field")));
    at_2(Some(t.hidden("hidden in an option")));
    at_2(vec![t.hidden("hidden in a vec")]);
    relay(t.hidden("hidden relayed"));
    relay_same(t.hidden("hidden relayed by a bound"));
    println!("end");
}

fn main() {
    paths(&Louds);
    paths(&Quiet);
}
