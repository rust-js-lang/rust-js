// An adapter held in a variable, then given to adapters made in place and
// stepped from its back by their `rev()` (#790): a chain of the crate's
// iterators, mapped, and filtered then mapped, a `Vec`'s `skip` mapped and
// a `Vec`'s map filtered. Nothing else in the crate steps their types from
// their backs: the held one is made knowing its back where it's made, as one
// given held to the `rev()` itself is (#484).

struct Own(u32, &'static str);
impl Drop for Own {
    fn drop(&mut self) {
        println!("drop {}", self.1);
    }
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
}

fn main() {
    let c = Own(2, "a").chain(Own(2, "b"));
    let v: Vec<u32> = c.map(|x| x + 10).rev().collect();
    println!("{v:?}");
    let c = Own(3, "c").chain(Own(1, "d"));
    let v: Vec<u32> = c.filter(|x| x % 2 == 0).map(|x| x * 3).rev().collect();
    println!("{v:?}");
    let s = vec![1u32, 2, 3, 4, 5].into_iter().skip(1);
    let v: Vec<u32> = s.map(|x| x + 1).rev().collect();
    println!("{v:?}");
    let m = vec![1u32, 2, 3].into_iter().map(|x| x * 2);
    let v: Vec<u32> = m.filter(|x| *x > 2).rev().collect();
    println!("{v:?}");
}
