//@ library-differs
// `{:?}` of a `Zip` is std's own `Debug`, compiled into `@rust-js/std`: its
// `ZipFmt` picked by its sides' facts, as std's specialization picks it by
// their types. Of two `TrustedRandomAccessNoCoerce` sides, slices'
// iterators, ranges, a `map` of one, a `Vec`'s, `Zip` alone, never what they
// hold, however far it was stepped; of any other, `Zip { a: .., b: .. }`,
// each side as stepping leaves it, through generic code, `{:#?}`, an
// `Option` and a zip of zips alike (#2252).
use std::fmt::Debug;

fn show<T: Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn main() {
    let mut pairs = [1, 2].iter().zip([3, 4].iter());
    println!("{:?}", pairs);
    pairs.next();
    println!("{:?} {:#?}", pairs, pairs);
    let v = [5, 6];
    let skipped = v.iter().zip(v.iter().skip(1));
    println!("{skipped:?}");
    println!("{}", skipped.count());
    let chunks = v.chunks(1).zip(v.iter());
    println!("{chunks:?}");
    let ranges = (0..3).zip(5..9);
    println!("{ranges:?} {}", show(&ranges));
    let mapped = [1, 2].iter().map(|x| x * 2).zip([3, 4].iter());
    println!("{mapped:?}");
    let mut chars = "ab".chars().zip("cd".chars());
    println!("{chars:?}");
    chars.next();
    println!("{chars:?}");
    println!("{chars:#?}");
    let mixed = "xy".chars().zip([7, 8].iter());
    println!("{mixed:?} {:?}", Some(&mixed));
    let mut owned = vec![1, 2, 3].into_iter().zip(vec!['a', 'b']);
    owned.next();
    println!("{owned:?}");
    let nested = (0..2).zip("pq".chars()).zip([true, false].iter());
    println!("{nested:?} {}", show(&nested));
}
