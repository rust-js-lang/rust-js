//@ compile-fail: `size_of` of `B`, which `<Measured as std::iter::Iterator>::collect` asks of its own `collect` and generic code's dispatch doesn't give an override (#2449)
// An iterator of the crate's whose own `collect` asks the size of what it's
// asked for, which generic code's call knows, `Vec<u8>`'s: refused where
// the iterator is given, naming the override and the fact, as generic
// code's dispatch doesn't give its overrides' entries a fact of the
// method's own parameters yet (#2449), never run as std's would.
struct Measured(u8);

impl Iterator for Measured {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        (self.0 <= 2).then_some(self.0)
    }
    fn collect<B: FromIterator<u8>>(self) -> B {
        println!("own collect into {} bytes", std::mem::size_of::<B>());
        std::iter::once(7).collect()
    }
}

fn gathered_bytes<I: Iterator<Item = u8>>(it: I) -> Vec<u8> {
    it.collect()
}

fn gathered_byte_set<I: Iterator<Item = u8>>(it: I) -> std::collections::BTreeSet<u8> {
    it.collect()
}

fn main() {
    println!("{:?}", gathered_bytes(Measured(0)));
    println!("{:?}", gathered_byte_set(Measured(0)));
}
