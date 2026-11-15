// A `Vec` std grows by an iterator's `size_hint()` lower bound, in a
// program that reads a `Vec`'s capacity: of a string's `chars()`
// enumerated, which passes on `chars()`' `((len + 3) / 4, Some(len))`; and
// of a slice's split by a predicate, `(1, Some(len + 1))` till it's done,
// which runs no code here, `u8::is_ascii_whitespace`, and was the array of
// its pieces, whose hint was its length. Each was refused beside a program
// that reads a capacity (#1892); each is std's hint now, fresh and part
// stepped.
fn main() {
    let numbered: Vec<(usize, char)> = "héllo wörld".chars().enumerate().collect();
    println!("{} {}", numbered.len(), numbered.capacity());
    let mut stepped = "abcdefghij".chars().enumerate();
    stepped.next();
    stepped.next();
    let rest: Vec<_> = stepped.collect();
    println!("{} {}", rest.len(), rest.capacity());

    let bytes = b"a bb  ccc d";
    let words: Vec<&[u8]> = bytes.split(u8::is_ascii_whitespace).collect();
    println!("{} {}", words.len(), words.capacity());
    let mut split = bytes.split(u8::is_ascii_whitespace);
    split.next();
    let later: Vec<&[u8]> = split.collect();
    println!("{} {}", later.len(), later.capacity());
    let limited: Vec<&[u8]> = bytes.splitn(3, u8::is_ascii_whitespace).collect();
    println!("{} {}", limited.len(), limited.capacity());
    let backwards: Vec<&[u8]> = bytes.rsplit(u8::is_ascii_whitespace).collect();
    println!("{} {}", backwards.len(), backwards.capacity());
    let kept: Vec<&[u8]> = bytes.split_inclusive(u8::is_ascii_whitespace).collect();
    println!("{} {} {:?}", kept.len(), kept.capacity(), kept[0]);
    let empty: Vec<&[u8]> = b"".split(u8::is_ascii_whitespace).collect();
    println!("{} {}", empty.len(), empty.capacity());
    for text in [&b"a b c d e"[..], b"a b c d e f", b"a b c d e f g h i"] {
        let lens: Vec<usize> = text.split(u8::is_ascii_whitespace).map(|s| s.len()).collect();
        let skipped: Vec<&[u8]> = text.split(u8::is_ascii_whitespace).skip(1).collect();
        let taken: Vec<&[u8]> = text.split(u8::is_ascii_whitespace).take(4).collect();
        let mut grown = Vec::new();
        grown.extend(text.split(u8::is_ascii_whitespace));
        println!(
            "{} {} {} {} {} {} {} {}",
            lens.len(),
            lens.capacity(),
            skipped.len(),
            skipped.capacity(),
            taken.len(),
            taken.capacity(),
            grown.len(),
            grown.capacity()
        );
    }
}
