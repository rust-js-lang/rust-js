// A `flat_map` or a `flatten()` of items a where clause says are
// `IntoIterator`, `Self::Item: IntoIterator` of a blanket impl's default
// copied into its impl, with items that have a destructor: each item made
// an iterator by its dictionary's `into_iter`, where it was stepped as if
// it were one and threw, `it.next is not a function` (#829). What each
// inner iterator leaves is dropped as it's closed, as Rust drops a
// `Flatten`'s.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Ext: Iterator {
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
}

impl<T: Iterator + ?Sized> Ext for T {}

fn ds(numbers: &[u32]) -> Vec<D> {
    numbers.iter().map(|&n| D(n)).collect()
}

fn numbers(v: &[D]) -> Vec<u32> {
    v.iter().map(|d| d.0).collect()
}

fn main() {
    let flat = vec![ds(&[1]), ds(&[2, 3])].into_iter().flat();
    println!("flat {:?}", numbers(&flat));
    drop(flat);
    let flattened = vec![ds(&[4, 5]), ds(&[]), ds(&[6])].into_iter().flattened();
    println!("flattened {:?}", numbers(&flattened));
    drop(flattened);
    let second = vec![ds(&[7]), ds(&[8, 9])].into_iter().first_of_each();
    println!("second {:?}", second.as_ref().map(|d| d.0));
    drop(second);
    let sums: Vec<u32> = vec![vec![1u32, 2], vec![3]].into_iter().flat();
    println!("sums {sums:?}");
}
