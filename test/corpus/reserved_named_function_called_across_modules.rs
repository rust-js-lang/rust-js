//@ library-differs
// A function of another module named by a word JS keeps, `new`, `delete`,
// `class`, `function`, `switch`, or a global its module may read, `fetch`,
// is named apart there, `new$`, and exported by its own name, `export {
// new$ as new }`; each call of it from another module reads that export,
// a trait default's copy alike the function itself too, `Lazy.new(..)`,
// where it read `Lazy.new$`, which the module doesn't export, and threw
// (#1390, itertools' `groupbylazy::new` of `chunk_by`).
mod lazy {
    pub struct Group<I, F> {
        pub iter: I,
        pub f: F,
    }

    pub fn new<J: IntoIterator, F: FnMut(&J::Item) -> u32>(iter: J, f: F) -> Group<J::IntoIter, F> {
        Group {
            iter: iter.into_iter(),
            f,
        }
    }

    pub fn delete<J: IntoIterator>(iter: J) -> usize {
        iter.into_iter().count()
    }

    pub fn class<J: IntoIterator<Item = u32>>(iter: J) -> u32 {
        iter.into_iter().sum()
    }

    pub fn function(n: u32) -> u32 {
        n * 3
    }

    pub fn switch(on: bool) -> &'static str {
        if on { "on" } else { "off" }
    }

    pub fn fetch<J: IntoIterator<Item = u32>>(iter: J) -> Option<u32> {
        iter.into_iter().max()
    }
}

pub trait Tools: Iterator {
    fn grouped<F: FnMut(&Self::Item) -> u32>(self, f: F) -> lazy::Group<Self, F>
    where
        Self: Sized,
    {
        lazy::new(self, f)
    }

    fn counted(self) -> usize
    where
        Self: Sized,
    {
        lazy::delete(self)
    }

    fn summed(self) -> u32
    where
        Self: Sized + Iterator<Item = u32>,
    {
        lazy::class(self)
    }

    fn greatest(self) -> Option<u32>
    where
        Self: Sized + Iterator<Item = u32>,
    {
        lazy::fetch(self)
    }
}

impl<T: Iterator> Tools for T {}

fn main() {
    let mut group = vec![1u32, 2].into_iter().grouped(|n| n * 10);
    let first = group.iter.next().unwrap();
    println!("{}", (group.f)(&first));
    let direct = lazy::new([5u32], |n| n + 1);
    println!("{}", direct.iter.count());
    println!("{} {}", vec![1, 2, 3].into_iter().counted(), lazy::delete([7]));
    println!("{} {}", (1..4).summed(), lazy::class([10, 20]));
    println!("{:?} {:?}", vec![4, 9, 2].into_iter().greatest(), lazy::fetch(None));
    println!("{} {}", lazy::function(2), lazy::switch(true));
}
