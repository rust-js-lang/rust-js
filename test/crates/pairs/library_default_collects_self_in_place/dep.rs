//! A library trait's defaults over its blanket impl's `Self` that collect
//! it into a `Vec`, as itertools' `collect_vec` and `sorted` do: std may
//! collect a `vec::IntoIter` in place, its buffer reused (#1370).
pub trait Tools: Iterator {
    fn collect_vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }

    fn sorted(self) -> std::vec::IntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v = Vec::from_iter(self);
        v.sort();
        v.into_iter()
    }
}

impl<T: Iterator> Tools for T {}
