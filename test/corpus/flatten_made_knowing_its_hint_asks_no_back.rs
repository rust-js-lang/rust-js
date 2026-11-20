// What's made knowing its hint is never made knowing its back for it
// (#2209): a flatten of the crate's iterators, which have no back, beside
// another of its type whose hint is asked, is `$flatMap`, whose hint asks
// each inner's own, not `$flattenEnds`, which stepped each inner by its
// back and threw.
struct Own(u32, u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("own size_hint {}", self.0);
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

struct Bare(u32);

impl Iterator for Bare {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
}

fn hint<I: Iterator<Item = u32>>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn main() {
    let mut it = vec![Own(0, 2), Own(0, 1)].into_iter().flatten();
    it.next();
    let v: Vec<u32> = it.collect();
    println!("flatten collect {v:?}");
    let mut other = vec![Own(0, 2), Own(0, 1)].into_iter().flatten().map(|x| x + 1);
    other.next();
    println!("map hint {:?}", other.size_hint());
    // Of the crate's iterators with no hint of their own.
    let mut bare = vec![Bare(2), Bare(1)].into_iter().flatten();
    bare.next();
    println!("bare {:?}", bare.collect::<Vec<_>>());
    let mut mapped = vec![Bare(2), Bare(1)].into_iter().flatten().map(|x| x * 2);
    mapped.next();
    println!("bare map hint {:?}", mapped.size_hint());
    // Given to generic code that asks its hint, and beside it.
    println!("given {:?}", hint(vec![Bare(2), Bare(1)].into_iter().flatten()));
    println!(
        "given map {:?}",
        hint(vec![Bare(3)].into_iter().flatten().map(|x| x + 1))
    );
    println!(
        "given filter {:?}",
        hint(vec![Bare(3)].into_iter().flatten().filter(|x| x % 2 == 0))
    );
    let beside: Vec<u32> = vec![Bare(3)].into_iter().flatten().filter(|x| x % 2 == 0).collect();
    println!("beside {beside:?}");
    println!(
        "given chain {:?}",
        hint(vec![1u32, 2].into_iter().chain(vec![Bare(2)].into_iter().flatten()))
    );
}
