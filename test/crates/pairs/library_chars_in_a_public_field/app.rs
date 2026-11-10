pub fn main() {
    for s in ["", "héllo", "日本語😀"] {
        let mut at = dep::cursor(s);
        println!("{} {:?} {:?}", at.read, at.chars.next_back(), at.chars.next());
    }
}
