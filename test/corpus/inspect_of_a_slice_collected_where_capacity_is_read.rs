// A `Vec` collected of a slice's iterator through an `inspect()`, in a crate
// that reads capacities: std's `Inspect` of a slice's knows its length, so
// `from_iter` reserves it after the first item, 5 of 5. Made in place,
// `items.values().map(..)`, JS's own, it knows no hint, and grew from 4 to
// 8 (#1792): it's given knowing its length, `$mapEnds` of the slice's
// `$iter`. So is an `inspect()` past a `map()`, one an `enumerate()` holds,
// and one of a `Vec<String>`'s.

fn main() {
    let items = [1u32, 2, 3, 4, 5];
    let seen: Vec<&u32> = items.iter().inspect(|x| print!("{x} ")).collect();
    println!("inspect {:?} capacity {}", seen, seen.capacity());
    let mapped: Vec<u32> = items.iter().map(|x| x * 10).inspect(|_| ()).collect();
    println!("map inspect {:?} capacity {}", mapped, mapped.capacity());
    let counted: Vec<(usize, &u32)> = items.iter().inspect(|_| ()).enumerate().collect();
    println!("inspect enumerate {:?} capacity {}", counted, counted.capacity());
    let words: Vec<String> = ["a", "bb", "ccc", "dddd", "eeeee", "f"]
        .iter()
        .map(|w| w.to_string())
        .collect();
    let lengths: Vec<&String> = words.iter().inspect(|w| print!("{} ", w.len())).collect();
    println!("strings {:?} capacity {}", lengths, lengths.capacity());
    let filtered: Vec<&u32> = items.iter().inspect(|_| ()).filter(|x| **x > 2).collect();
    println!("filtered {:?} room {}", filtered, filtered.capacity() >= filtered.len());
}
