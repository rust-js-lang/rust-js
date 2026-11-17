// #1576: `default_where_clause_item_drops`'s crate beside an iterator of
// its own with a `size_hint`, given to `Vec::extend`: the hint makes a
// dictionary carry its methods' type facts, and `Range<u8>`'s carries
// those of the default `held_all`, whose `where Self::Item: IntoIterator`
// `u8` can't meet, of `Held<<u8 as IntoIterator>::IntoIter>`, no type
// here: nothing can call that method, so its slot is given nothing, as
// one of a projection alone is (`drops::no_type_here`, #798).
// A trait's default, copied into a blanket impl, drops a value of an
// associated type its own where clause names, `<Self::Item as
// IntoIterator>::IntoIter` of a `Self::Item: IntoIterator`, which no bound
// of the impl names: given its drop as the impl's are (ADR 0478, #798).

struct D(u64);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I> {
    iter: I,
}

impl<I> Drop for Held<I> {
    fn drop(&mut self) {
        println!("drop held");
    }
}

trait Holding: Iterator {
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

    fn first_held(mut self) -> Option<Held<<Self::Item as IntoIterator>::IntoIter>>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.next().map(|i| Held { iter: i.into_iter() })
    }

    // Of one whose items aren't `IntoIterator`, given the impl's dictionary
    // too.
    fn counted(self) -> usize
    where
        Self: Sized,
    {
        self.count()
    }
}

impl<T: Iterator> Holding for T {}

// A default of its own type parameter too, bound by a trait of associated
// types, which aren't the impl's.
trait Joining: Iterator {
    fn held_with<U: IntoIterator>(self, extra: U) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let mut all = Vec::new();
        for i in self {
            all.push(Held { iter: i.into_iter() });
        }
        all.len() + extra.into_iter().count()
    }
}

impl<T: Iterator> Joining for T {}

fn wide(w: Option<D>) -> Option<D> {
    if w.as_ref().is_some_and(|d| d.0 == 3) {
        panic!("wide");
    }
    w
}

// Through generic code, which gives the default's drop as its caller gave it.
fn held_through<I: Iterator<Item = Option<D>>>(iter: I) -> usize {
    iter.held_all()
}

// And of a caller's own type parameter's item.
fn held_of<I>(iter: I) -> usize
where
    I: Iterator,
    I::Item: IntoIterator,
{
    iter.held_all()
}

fn victim_main() {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let wides: Vec<Option<D>> = (1..=4).map(|n| Some(D(n))).collect();
        wides.into_iter().map(wide).held_all()
    }));
    println!("{:?}", r.ok());

    let wides: Vec<Option<D>> = (5..=6).map(|n| Some(D(n))).collect();
    println!("{}", wides.into_iter().held_all());

    let nested: Vec<Vec<D>> = vec![vec![D(7), D(8)], vec![D(9)]];
    println!("{}", held_through(nested.into_iter().map(|v| v.into_iter().next())));

    let nested: Vec<Vec<D>> = vec![vec![D(10)], vec![D(11), D(12)]];
    println!("{}", held_of(nested.into_iter()));

    let plain: Vec<Vec<u8>> = vec![vec![1, 2], vec![3]];
    println!("{}", held_of(plain.into_iter()));

    let first = vec![Some(D(13)), Some(D(14))].into_iter().first_held();
    println!("first {:?}", first.as_ref().map(|h| h.iter.len()));
    drop(first);
    println!("{}", (0..3u8).counted());
    println!("{}", vec![D(15)].into_iter().counted());
    println!("{}", vec![Some(D(16))].into_iter().held_with(vec![17u8]));
    println!("end");
}

struct Count(u32);
impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, None)
    }
}
struct Noisy(u32);
fn main() {
    victim_main();
    let mut v: Vec<Noisy> = Vec::new();
    v.extend(Count(3).map(Noisy));
    println!("{} {}", v.len(), v.iter().map(|n| n.0).sum::<u32>());
}
