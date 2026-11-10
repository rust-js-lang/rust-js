pub fn main() {
    for s in ["", "héllo", "日本語😀"] {
        let mut it = dep::letters(s);
        println!("{:?} {:?}", it.next_back(), it.collect::<String>());
    }
}
