// In a crate that reads a `Vec`'s capacity, a `Vec` grown by an iterator
// whose adapters hold others, a `take(n)` of a `cycle()` of a `map`,
// collected and extended. Std grows it by its hint, so each adapter in it
// is made knowing its own, and the capacity is std's (#1572).

fn main() {
    let cycled: Vec<u32> = [1u32, 2].iter().map(|x| x + 1).cycle().take(5).collect();
    println!("{cycled:?} {}", cycled.capacity());
    let mut grown = Vec::with_capacity(1);
    grown.extend([4u32, 5].iter().map(|x| x * 3).cycle().take(3));
    println!("{grown:?} {}", grown.capacity());
}
