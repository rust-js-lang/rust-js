// A `String` changed in place, by its UTF-8 byte offsets, as Rust's is: JS
// strings don't change, so its place is given the new one.

fn tidy(s: &mut String) -> Option<char> {
    s.retain(|c| !c.is_whitespace());
    s.insert_str(0, "[");
    s.push(']');
    s.pop()
}

fn main() {
    let mut s = String::with_capacity(8);
    s.push_str("héllo🎉");
    println!("{:?} {s:?}", s.pop());
    println!("{:?} {s:?}", s.pop());
    s.insert(0, '¡');
    s.insert_str(3, "--");
    println!("{s}");
    println!("{} {s}", s.remove(2));
    println!("{} {s}", s.remove(0));
    s.truncate(4);
    println!("{s:?}");
    s.truncate(9);
    println!("{s:?}");
    s.retain(|c| c != '-');
    println!("{s:?}");
    s.clear();
    println!("{s:?} {} {:?}", s.is_empty(), s.pop());
    let mut t = String::from(" a b c ");
    let last = tidy(&mut t);
    println!("{last:?} {t:?}");
    let mut words = vec![String::from("one"), String::from("two")];
    words[1].insert(0, '2');
    words[0].truncate(1);
    println!("{words:?}");
}
