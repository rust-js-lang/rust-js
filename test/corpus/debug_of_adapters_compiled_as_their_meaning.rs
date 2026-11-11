// `{:?}` of std's adapters compiled as their meaning alone, `filter`'s,
// `filter_map`'s, `take_while`'s, `skip_while`'s, `map_while`'s, `scan`'s,
// `copied()`'s and `cloned()`'s, is std's text, the closure left out (#1401):
// only std's struct shows one, so each type whose `{:?}` the crate asks is
// std's struct where it's held, its own `Debug` compiled into `@rust-js/std`,
// as a `map()` held is; every other value of the adapter stays the
// recognizers', `range.filter(p)`. Through a generic wrapper's derived
// `Debug`, as itertools' `ExactlyOneError<Filter<..>>` is shown by `unwrap`.
#[derive(Debug)]
struct Leftover<I> {
    first: Option<u32>,
    rest: I,
}

fn exactly_one<I: Iterator<Item = u32>>(mut it: I) -> Result<u32, Leftover<I>> {
    match (it.next(), it.next()) {
        (Some(one), None) => Ok(one),
        (first, _) => Err(Leftover { first, rest: it }),
    }
}

fn main() {
    let f = (0..10).filter(|&x| x == 2);
    println!("{f:?}");
    println!("{:?}", (0..4u32).filter_map(|x| x.checked_sub(1)));
    println!("{:?}", (0..4).take_while(|x| *x < 2));
    println!("{:?}", (0..4).skip_while(|x| *x < 2));
    println!("{:?}", (0..4u32).map_while(|x| x.checked_sub(1)));
    println!(
        "{:?}",
        (0..4).scan(0, |sum, x| {
            *sum += x;
            Some(*sum)
        })
    );
    println!("{:?}", [1, 2].iter().copied());
    println!("{:?}", [String::from("a")].iter().cloned());
    println!("{}", exactly_one((0..10u32).filter(|&x| x == 2)).unwrap());
    println!("{:?}", exactly_one((0..10u32).filter(|&x| x > 1 && x < 5)).unwrap_err());
    // Another filter, whose `{:?}` nothing asks, the recognizers'.
    let mut other = (0..10).filter(|&x| x % 4 == 1);
    println!("{:?} {:?}", other.next(), other.next());
    let total: i32 = (0..10).filter(|x| x % 3 == 0).sum();
    println!("{total}");
}
