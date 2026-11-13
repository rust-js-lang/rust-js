//! A library's trait whose defaults a where clause bounds, copied into its
//! blanket impl, whose dictionary it exports: each asks facts of its own
//! slots, and the impl the layout of what its items hold (#1612).
pub struct D(pub u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub trait Ext: Iterator {
    fn flat(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).collect()
    }

    fn flattened(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flatten().collect()
    }

    fn first_of_each(self) -> Option<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).nth(1)
    }

    fn hint(self) -> (usize, Option<usize>)
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).size_hint()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

pub fn ds(numbers: &[u32]) -> Vec<D> {
    numbers.iter().map(|&n| D(n)).collect()
}

pub fn numbers(v: &[D]) -> Vec<u32> {
    v.iter().map(|d| d.0).collect()
}
