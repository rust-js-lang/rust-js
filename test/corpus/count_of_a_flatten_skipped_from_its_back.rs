// A `skip(1).count()` in generic code of a flatten stepped from its back,
// whose inner iterators have their own `count` and a destructor: std's
// `FlattenCompat::count`, each inner's own `count`, never a `len()` the
// flatten hasn't, though it has an `advance_back_by` of its own, as a
// `Vec`'s `IntoIter` does (#2154). And its `last`, and the same of a `Vec`'s
// `IntoIter`, whose own `count` is its length.
struct Own(u32);

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }

    fn count(self) -> usize {
        println!("own count {}", self.0);
        self.0 as usize
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own {}", self.0);
    }
}

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

fn skipped_from_its_back<I: DoubleEndedIterator>(mut it: I) -> usize
where
    I::Item: std::fmt::Debug,
{
    println!("back {:?}", it.next_back());
    it.skip(1).count()
}

fn last_from_its_back<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back();
    it.skip(1).last()
}

fn main() {
    let flat = vec![2u32, 3, 4].into_iter().flat_map(Own);
    println!("back skipped {}", skipped_from_its_back(flat));
    let flat = vec![2u32, 3, 4].into_iter().flat_map(Own);
    println!("last {:?}", last_from_its_back(flat));
    let louds = vec![Loud(1), Loud(2), Loud(3), Loud(4)].into_iter();
    println!("louds skipped {}", skipped_from_its_back(louds));
    let louds = vec![Loud(5), Loud(6), Loud(7)].into_iter();
    println!("last loud {:?}", last_from_its_back(louds).map(|loud| loud.0));
}
