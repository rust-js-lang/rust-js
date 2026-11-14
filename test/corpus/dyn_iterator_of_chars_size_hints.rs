// In a crate that asks a `dyn Iterator`'s hint, a `dyn` made of what keeps
// std's hint by its form over a string's `chars()` or `char_indices()`, a
// `map` of one, its `rev()`'s `map`: each `chars()` is the runtime's
// `$chars`, whose hint is std's, `(len + 3) / 4` of its bytes, and what's
// made of it passes it on (#1563).

fn hints(mut it: Box<dyn Iterator<Item = u32> + '_>) {
    print!("{:?}", it.size_hint());
    it.next();
    println!(" {:?}", it.size_hint());
}

fn main() {
    for s in ["héllo wörld, ünïcödé", "abcdefgh", "", "日本"] {
        hints(Box::new(s.chars().map(|c| c as u32)));
        hints(Box::new(s.char_indices().map(|(i, _)| i as u32)));
        hints(Box::new(s.chars().rev().map(|c| c as u32)));
    }
}
