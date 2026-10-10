// A `&mut str` written through, as `make_ascii_uppercase` writes one: of a
// `String`, a `Box<str>`, an item, a range of one, and a part `get_mut`
// and `split_at_mut` give, each the same bytes changed in place.

fn shout(t: &mut str) {
    t.make_ascii_uppercase();
}

fn upper(t: &mut String) {
    t.make_ascii_uppercase();
}

struct Holder<'a> {
    text: &'a mut str,
}

fn tail<'a>(s: &'a mut String) -> &'a mut str {
    &mut s[7..]
}

fn main() {
    let mut s = String::from("héllo wörld");
    s.make_ascii_uppercase();
    println!("{s}");
    s.as_mut_str().make_ascii_lowercase();
    println!("{s}");
    shout(&mut s);
    println!("{s}");
    let t: &mut str = &mut s;
    t.make_ascii_lowercase();
    println!("{s}");
    let again: &mut str = &mut s;
    shout(again);
    println!("{s}");
    s.make_ascii_lowercase();

    let mut b: Box<str> = Box::from("boxed");
    b.make_ascii_uppercase();
    println!("{b}");

    let mut u = String::from("abc");
    upper(&mut u);
    println!("{u}");

    let mut v = vec![String::from("x"), String::from("y")];
    v[1].make_ascii_uppercase();
    shout(&mut v[0]);
    for w in v.iter_mut() {
        w.make_ascii_lowercase();
    }
    println!("{v:?}");

    let mut kept = String::from("hey");
    let h = Holder { text: &mut kept };
    h.text.make_ascii_uppercase();
    println!("{kept}");

    // A range of one, by its UTF-8 bytes.
    let mut name = String::from("élan vital");
    name[2..4].make_ascii_uppercase();
    println!("{name}");
    name[..].make_ascii_lowercase();
    println!("{name}");
    shout(&mut name[..4]);
    tail(&mut name).make_ascii_uppercase();
    println!("{name}");

    let mut word = String::from("capital");
    if let Some(head) = word.get_mut(0..1) {
        head.make_ascii_uppercase();
    }
    println!("{word}");
    println!("{:?}", word.get_mut(1..9));
    println!("{:?}", word.get_mut(9..));
    let mut accented = String::from("ébc");
    println!("{:?}", accented.get_mut(1..).is_none());

    let mut pair = String::from("left|right");
    let (a, z) = pair.split_at_mut(5);
    a.make_ascii_uppercase();
    z.make_ascii_uppercase();
    a.make_ascii_lowercase();
    println!("{a} {z}");
    println!("{pair}");
    println!("{:?}", pair.split_at_mut_checked(11).is_none());
    if let Some((_, rest)) = pair.split_at_mut_checked(4) {
        rest.make_ascii_lowercase();
    }
    println!("{pair}");

    unsafe {
        println!("{}", pair.get_unchecked(1..3));
        println!("{}", pair.slice_unchecked(2, 5));
        pair.get_unchecked_mut(0..2).make_ascii_uppercase();
        pair.slice_mut_unchecked(6, 8).make_ascii_uppercase();
    }
    println!("{pair}");
    let head = unsafe { pair.slice_mut_unchecked(0, 3) };
    head.make_ascii_lowercase();
    println!("{pair}");
    pair[7..].make_ascii_uppercase();
    println!("{pair}");
    pair[..2].make_ascii_uppercase();
    println!("{pair}");

    // A `&mut str` of bytes, written through to them.
    let mut bytes = *b"hello";
    if let Ok(text) = std::str::from_utf8_mut(&mut bytes) {
        text.make_ascii_uppercase();
        println!("{text}");
    }
    println!("{bytes:?}");
    let mut bad = [0xffu8, b'a'];
    println!("{:?}", std::str::from_utf8_mut(&mut bad).is_err());
    let mut raw = vec![b'a', 0xc3, 0xa9, b'b'];
    unsafe {
        std::str::from_utf8_unchecked_mut(&mut raw).make_ascii_uppercase();
    }
    println!("{raw:?}");
}
