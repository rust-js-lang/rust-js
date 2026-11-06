// Generic code calling a blanket impl's default, `i.into_iter().first()`,
// where another default's where clause, `Self::Item: IntoIterator`, names
// an associated type its arguments there don't have: `<<I::IntoIter as
// Iterator>::Item as IntoIterator>::Item` is no type there, so the impl's
// slot of it, its drop and its size, is given nothing, as nothing called
// there reads it (#828). itertools' `intersperse`, `join` and `sorted`,
// beside its `flatten`-like defaults, so.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Ext: Iterator {
    fn first(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next()
    }

    fn flat(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let mut out = Vec::new();
        for items in self {
            for item in items {
                out.push(item);
            }
        }
        out
    }

    fn firsts(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.filter_map(|items| items.into_iter().next()).collect()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

fn first_of<I: IntoIterator>(items: I) -> Option<I::Item> {
    items.into_iter().first()
}

fn main() {
    let first = first_of(vec![D(1), D(2), D(3)]);
    println!("first {:?}", first.as_ref().map(|d| d.0));
    drop(first);
    let flat = vec![vec![D(4)], vec![D(5), D(6)]].into_iter().flat();
    println!("flat {:?}", flat.iter().map(|d| d.0).collect::<Vec<_>>());
    drop(flat);
    let firsts = vec![vec![D(7), D(8)], vec![], vec![D(9)]].into_iter().firsts();
    println!("firsts {:?}", firsts.iter().map(|d| d.0).collect::<Vec<_>>());
}
