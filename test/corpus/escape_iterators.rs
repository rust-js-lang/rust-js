// A `char`'s or a `str`'s escape is an iterator of its `char`s: collected,
// counted, stepped by a `for` and flat-mapped, as its text is shown.
fn main() {
    let shown: String = 'é'.escape_default().collect();
    let parts: Vec<char> = '\n'.escape_debug().collect();
    let length = 'é'.escape_unicode().count();
    println!("{shown} {parts:?} {length}");
    for c in 'x'.escape_unicode() {
        print!("{c}");
    }
    println!();
    let quoted: String = "a\n\"é".escape_debug().collect();
    let tabs = "tab\t".escape_default().count();
    let marks: String = "x\u{301}".chars().flat_map(char::escape_unicode).collect();
    println!("{quoted} {tabs} {marks}");
}
