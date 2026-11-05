// A library's trait default, copied into its blanket impl, holding a value
// of an associated type only its own where clause names: a consumer giving
// the impl's dictionary gives its drop too (#798).
pub struct Held<I> {
    pub iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

pub trait Holding: Iterator {
    fn held_all(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let mut all = Vec::new();
        for i in self {
            all.push(Held { iter: i.into_iter() });
        }
        all.len()
    }
}

impl<T: Iterator> Holding for T {}

// And one its consumer implements, whose default is the library's.
pub trait Holds: Iterator {
    fn held_each(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let mut all = Vec::new();
        for i in self {
            all.push(Held { iter: i.into_iter() });
        }
        all.len()
    }
}
