// A string's `chars()` and `char_indices()` asked their `size_hint()`
// (#1186): std's, of the bytes each has left, `(bytes.div_ceil(4),
// Some(bytes))`, as each is stepped from its front and its back. It was
// refused where `chars()` was an array of every `char`, whose hint was how
// many it had.

fn show(name: &str, s: &str) {
    let mut chars = s.chars();
    print!("{name}: {:?}", chars.size_hint());
    chars.next_back();
    print!(" {:?}", chars.size_hint());
    chars.next();
    println!(" {:?} {:?}", chars.size_hint(), chars.as_str());

    let mut indices = s.char_indices();
    indices.next();
    print!("  indices: {:?}", indices.size_hint());
    indices.next_back();
    println!(" {:?} at {}", indices.size_hint(), indices.offset());
}

fn main() {
    show("empty", "");
    show("one", "a");
    show("accents", "héllo wörld");
    show("kana", "日本語のテキスト");
    show("emoji", "a😀b😀c😀d😀e");
    show("ascii", "abcdefghijklmnopqrstuvwxyz0123456789");
    let lower: usize = ["xy", "ü", "語😀"].iter().map(|s| s.chars().size_hint().0).sum();
    println!("lower bounds: {lower}");
}
