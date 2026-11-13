// A comparator that orders strings by their UTF-16 units, JS's own order,
// is JS's `sort()` only where it does nothing else: one that counts its
// calls too keeps its closure, so the count is Rust's. It was `.sort()`,
// and the count stayed 0 (#1645).
fn main() {
    let mut words: Vec<String> = ["b", "a", "c", "a"].iter().map(|w| w.to_string()).collect();
    let mut calls = 0;
    words.sort_unstable_by(|a, b| {
        calls += 1;
        a.encode_utf16().cmp(b.encode_utf16())
    });
    println!("{calls} {words:?}");

    let mut stable: Vec<&str> = vec!["d", "b", "a", "c"];
    let mut seen = Vec::new();
    stable.sort_by(|a, b| {
        seen.push((*a, *b));
        a.encode_utf16().cmp(b.encode_utf16())
    });
    println!("{} {stable:?}", seen.len());

    let mut pure: Vec<&str> = vec!["b", "a", "c"];
    pure.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    println!("{pure:?}");
}
