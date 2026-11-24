//@ run-fail: iterator is infinite
// An adapter's site, `skip`'s line, held while its `count` runs, is put back
// as its panic unwinds, so a later `repeat()` counted directly names its own
// caller, never `skip.rs`; and one counted inside another's closure, by a
// `map` of what a `skip` counts, names its own (#2488).
use std::iter::repeat;
use std::panic::catch_unwind;

fn main() {
    println!("{}", catch_unwind(|| repeat(1u8).skip(1).count()).is_err());
    println!("{}", catch_unwind(|| repeat(1u8).enumerate().count()).is_err());
    let inner = catch_unwind(|| {
        (0..2u8)
            .skip(1)
            .map(|_| catch_unwind(|| repeat(2u8).count()).is_err())
            .count()
    });
    println!("{:?}", inner.is_ok());
    println!("{}", repeat(3u8).count());
}
