// A trait's default copied into an impl whose `Self::Item` is one of the
// impl's own type parameters, `impl<I, T> Tools for W<I> where I:
// Iterator<Item = T>`, gives `Self` to a generic function that grows a
// `Vec` of its items and reads its capacity: the size of `<W<I> as
// Iterator>::Item` it's given, `T`'s as the impl's bounds normalize it
// (#2012). Items of one byte, of 320 and with a destructor.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

fn room<J: Iterator>(it: J) -> usize {
    let mut v = Vec::new();
    for x in it {
        v.push(x);
    }
    v.capacity()
}

trait Tools: Iterator {
    fn roomed(self) -> usize
    where
        Self: Sized,
    {
        room(self)
    }
}

struct W<I>(I);

impl<I: Iterator<Item = T>, T> Iterator for W<I> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
}

impl<I, T> Tools for W<I> where I: Iterator<Item = T> {}

fn main() {
    println!("{}", W(vec![1u8, 2, 3, 4, 5].into_iter()).roomed());
    println!("{}", W(vec![[0u64; 40]; 3].into_iter()).roomed());
    println!("{}", W(vec![Loud(1), Loud(2)].into_iter()).roomed());
}
