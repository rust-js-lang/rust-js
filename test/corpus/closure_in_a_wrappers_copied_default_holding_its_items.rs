//@ compile-fail: a closure that holds a value with a destructor, made here
// A closure in a copied default of an impl that isn't a blanket one,
// holding a `Vec<Self::Item>`, `<Wrapper<I> as Iterator>::Item` in its
// impl's terms, is asked what it drops normalized, `I::Item`'s, as a
// blanket impl's is: one made in a tuple, where JS can't see what it took,
// is refused, as it is in a generic function. Asked as the alias, which the
// drop slots take for one dropping nothing, it compiled, right only by luck
// (#1337).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Ext: Iterator {
    fn counted(self) -> usize
    where
        Self: Sized,
    {
        let front: Vec<Self::Item> = self.collect();
        let (count, extra) = (move || front.len(), 1);
        println!("made");
        count() + extra
    }
}

impl<I: Iterator> Ext for Wrapper<I> {}

fn main() {
    println!("{}", Wrapper((1..3).map(Noisy)).counted());
}
