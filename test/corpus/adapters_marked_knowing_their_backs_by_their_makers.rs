// Each std adapter the crate steps from its back, held in a variable
// before it's stepped: what makes it knowing its back marks it so, and that
// mark, carried by the variable, is what its steps from the back read. No
// mark is promised by the adapter's type alone, which a maker that made
// none so would have taken for a back (#2069, #2068). A map and a filter,
// an enumeration, a copy and a clone, an `inspect`, a `filter_map`, a
// chain, a flatten, a `rev()`: of the crate's own iterator, std's struct
// where it's held, and of a type parameter's, made where it's held, given
// the crate's iterator and an array's; each cloned apart, and a map whose
// function has a destructor, held to drop it, too.
#[derive(Clone)]
struct Walk {
    front: u32,
    back: u32,
}

impl Iterator for Walk {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.front < self.back).then(|| {
            self.front += 1;
            self.front - 1
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.back - self.front) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Walk {
    fn next_back(&mut self) -> Option<u32> {
        (self.front < self.back).then(|| {
            self.back -= 1;
            self.back
        })
    }
}

impl ExactSizeIterator for Walk {}

#[derive(Clone)]
struct Refs<'a> {
    items: &'a [u32],
    front: usize,
    back: usize,
}

impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        (self.front < self.back).then(|| {
            self.front += 1;
            &self.items[self.front - 1]
        })
    }
}

impl DoubleEndedIterator for Refs<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        (self.front < self.back).then(|| {
            self.back -= 1;
            &self.items[self.back]
        })
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn walk() -> Walk {
    Walk { front: 1, back: 6 }
}

fn refs(items: &[u32]) -> Refs<'_> {
    Refs {
        items,
        front: 0,
        back: items.len(),
    }
}

fn ends<T: std::fmt::Debug>(name: &str, mut it: impl DoubleEndedIterator<Item = T>) {
    let last = it.next_back();
    let first = it.next();
    let before_last = it.next_back();
    println!("{name}: {last:?} {first:?} {before_last:?}");
}

fn generic<'a, I, R>(it: I, refs: R)
where
    I: DoubleEndedIterator<Item = u32> + ExactSizeIterator + Clone,
    R: DoubleEndedIterator<Item = &'a u32> + Clone,
{
    let mut mapped = it.clone().map(|x| x * 3);
    println!("generic map {:?} {:?}", mapped.next_back(), mapped.next());
    let mut filtered = it.clone().filter(|x| x % 2 == 0);
    println!("generic filter {:?} {:?}", filtered.next_back(), filtered.next());
    let mut enumerated = it.clone().enumerate();
    println!("generic enumerate {:?} {:?}", enumerated.next_back(), enumerated.next());
    let mut copied = refs.clone().copied();
    println!("generic copied {:?} {:?}", copied.next_back(), copied.next());
    let mut cloned = refs.clone().cloned();
    println!("generic cloned {:?} {:?}", cloned.next_back(), cloned.next());
    let mut inspected = it.clone().inspect(|x| println!("generic saw {x}"));
    println!("generic inspect {:?} {:?}", inspected.next_back(), inspected.next());
    let mut kept = it.clone().filter_map(|x| (x > 2).then_some(x + 1));
    println!("generic filter_map {:?} {:?}", kept.next_back(), kept.next());
    let mut chained = it.clone().chain(refs.clone().copied());
    println!("generic chain {:?} {:?}", chained.next_back(), chained.next());
    let mut flat = it.clone().map(|x| 0..x).flatten();
    println!("generic flatten {:?} {:?}", flat.next_back(), flat.next());
    let mut reversed = it.clone().rev();
    println!("generic rev {:?} {:?}", reversed.next_back(), reversed.next());
    let loud = Loud(9);
    let mut held = it.map(move |x| {
        let loud = &loud;
        x + loud.0
    });
    println!("generic held {:?} {:?}", held.next_back(), held.next());
    drop(held);
    println!("generic end");
}

// Made where they're held, of an array's items: a map, a filter and an
// `inspect` whose function has a destructor, held to drop it; and a map, a
// filter and a chain cloned apart.
fn in_place(items: [u32; 4]) {
    let loud = Loud(7);
    let mut held = items.iter().map(move |x| {
        let loud = &loud;
        x + loud.0
    });
    println!("held {:?} {:?} {:?}", held.next_back(), held.next(), held.next_back());
    let loud = Loud(8);
    let mut kept = items.iter().filter(move |x| {
        let loud = &loud;
        **x != loud.0
    });
    println!("kept {:?} {:?}", kept.next_back(), kept.next());
    let loud = Loud(9);
    let mut seen = items.iter().inspect(move |x| {
        let loud = &loud;
        println!("saw {x} {}", loud.0)
    });
    println!("seen {:?} {:?}", seen.next_back(), seen.next());
    drop(seen);
    let mapped = items.iter().map(|x| x + 1);
    let mut again = mapped.clone();
    println!("map clone {:?} {:?}", again.next_back(), again.next());
    let mut mapped = mapped;
    println!(
        "map {:?} {:?} {:?}",
        mapped.next_back(),
        mapped.next(),
        mapped.next_back()
    );
    let filtered = items.iter().filter(|x| **x > 10);
    let mut again = filtered.clone();
    println!("filter clone {:?} {:?}", again.next_back(), again.next());
    let mut filtered = filtered;
    println!("filter {:?} {:?}", filtered.next_back(), filtered.next());
    let chained = items.iter().chain(items[1..].iter());
    let mut again = chained.clone();
    println!("chain clone {:?} {:?}", again.next_back(), again.next());
    let mut chained = chained;
    println!("chain {:?} {:?}", chained.next_back(), chained.next());
    println!("in place end");
}

fn main() {
    let items = [10, 20, 30, 40];
    generic(walk(), refs(&items));
    in_place(items);
    generic([1, 2, 3, 4, 5].into_iter(), items.iter());
    let mut mapped = walk().map(|x| x * 2);
    println!(
        "map {:?} {:?} {:?}",
        mapped.next_back(),
        mapped.next(),
        mapped.next_back()
    );
    let mut filtered = walk().filter(|x| x % 2 == 1);
    println!("filter {:?} {:?}", filtered.next_back(), filtered.next_back());
    let mut enumerated = walk().enumerate();
    println!("enumerate {:?} {:?}", enumerated.next_back(), enumerated.next());
    let mut copied = refs(&items).copied();
    println!("copied {:?} {:?}", copied.next_back(), copied.next());
    let mut cloned = refs(&items).cloned();
    println!("cloned {:?} {:?}", cloned.next_back(), cloned.next());
    let mut inspected = walk().inspect(|x| println!("saw {x}"));
    println!("inspect {:?} {:?}", inspected.next_back(), inspected.next());
    let mut kept = walk().filter_map(|x| (x != 4).then_some(x * 10));
    println!("filter_map {:?} {:?}", kept.next_back(), kept.next_back());
    let mut chained = walk().chain(refs(&items).copied());
    println!("chain {:?} {:?}", chained.next_back(), chained.next());
    let mut flat = walk().map(|x| 0..x).flatten();
    println!("flatten {:?} {:?}", flat.next_back(), flat.next());
    let mut reversed = walk().rev();
    println!("rev {:?} {:?}", reversed.next_back(), reversed.next());

    // Cloned apart, each clone stepped from its back as it is.
    let mapped = walk().map(|x| x + 100);
    let mut again = mapped.clone();
    println!("map clone {:?} {:?}", again.next_back(), again.next());
    ends("map clone's source", mapped);
    let chained = walk().chain(walk());
    let mut again = chained.clone();
    println!("chain clone {:?} {:?}", again.next_back(), again.next());
    ends("chain clone's source", chained);
    let copied = refs(&items).copied();
    let mut again = copied.clone();
    println!("copied clone {:?} {:?}", again.next_back(), again.next());
    ends("copied clone's source", copied);

    // A map whose function has a destructor, held to drop it.
    let loud = Loud(7);
    let mut held = walk().map(move |x| {
        let loud = &loud;
        x + loud.0
    });
    println!("held {:?} {:?}", held.next_back(), held.next());
    drop(held);
    println!("end");
}
