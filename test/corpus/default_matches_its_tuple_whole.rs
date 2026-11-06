// A blanket impl's default of its own type parameter, itertools'
// `collect_tuple`, matching what another default gives whole, `elt @
// Some(_)`, a binding whose subpattern moves nothing out: the whole moves
// into `elt` as a binding without one does, so a tuple of items with a
// destructor is matched, and dropped where it isn't kept (#828).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Tuple: Sized {
    type Item;
    fn taken<I: Iterator<Item = Self::Item>>(it: &mut I) -> Option<Self>;
}

impl<A> Tuple for (A, A) {
    type Item = A;
    fn taken<I: Iterator<Item = A>>(it: &mut I) -> Option<(A, A)> {
        Some((it.next()?, it.next()?))
    }
}

trait Ext: Iterator {
    fn listed(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }

    fn next_tuple<T>(&mut self) -> Option<T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: Tuple,
    {
        T::taken(self)
    }

    fn collect_tuple<T>(mut self) -> Option<T>
    where
        Self: Sized + Iterator<Item = T::Item>,
        T: Tuple,
    {
        match self.next_tuple() {
            elt @ Some(_) => match self.next() {
                Some(_) => None,
                None => elt,
            },
            _ => None,
        }
    }
}

impl<I: Iterator> Ext for I {}

fn main() {
    let pair: Option<(D, D)> = vec![D(1), D(2)].into_iter().collect_tuple();
    println!("{:?}", pair.as_ref().map(|(a, b)| (a.0, b.0)));
    let none: Option<(D, D)> = vec![D(3), D(4), D(5)].into_iter().collect_tuple();
    println!("{}", none.is_none());
    let listed = vec![D(6), D(7)].into_iter().listed();
    println!("{}", listed.len());
}
