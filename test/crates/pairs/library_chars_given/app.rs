use dep::{cursor, first, hint, indexed, last, letters, opaque};

pub fn main() {
    for s in ["", "héllo", "日本語😀"] {
        let mut chars = letters(s);
        println!("{:?} {:?}", chars.next_back(), chars.size_hint());
        chars.next();
        println!("{:?} {:?}", chars.as_str(), chars.clone().count());

        let mut pairs = indexed(s);
        println!("{:?} {:?}", pairs.next_back(), pairs.offset());

        let mut hidden = opaque(s);
        println!("{:?} {:?}", hidden.next_back(), hidden.collect::<String>());

        let mut at = cursor(s);
        at.chars.next();
        println!("{:?} {:?}", at.chars.as_str(), at.chars.next_back());

        println!("{:?} {:?}", hint(s.chars()), hint(letters(s)));
        println!("{:?} {:?} {:?}", last(s.chars()), last(letters(s)), first(s));
    }
}
