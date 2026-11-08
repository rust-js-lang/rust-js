// `{:#?}` of std's iterators that show what they have left: a slice's
// `Iter`, its slice on a line of its own as std's `debug_tuple` puts it; a
// string's `Chars`, `Chars(` and a pretty list; a string's `Drain`. And of
// a derived `Debug` that holds one, pretty as its caller asks, where any
// `{:#?}` in the crate refused it (#953).
#[derive(Debug)]
struct Window<'a> {
    rest: std::slice::Iter<'a, u32>,
    name: &'static str,
}

fn main() {
    let v = vec![1, 2, 3];
    let mut it = v.iter();
    it.next();
    println!("{:#?}", it);
    println!("{:?}", it);
    let mut c = "héllo".chars();
    c.next();
    println!("{:#?}", c);
    let mut text = String::from("abcdef");
    let d = text.drain(1..4);
    println!("{:#?}", d);
    let empty: [u8; 0] = [];
    println!("{:#?}", empty.iter());
    let window = Window { rest: v[1..].iter(), name: "w" };
    println!("{window:?}");
    println!("{window:#?}");
}
