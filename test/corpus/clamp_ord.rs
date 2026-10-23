// clamp() of what Ord orders but JS's < doesn't, text, a char, a derived
// enum, a tuple: by its Ord, as std's is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Size { S, M, L }
fn main() {
    println!("{} {} {}", 'm'.clamp('a', 'f'), "q".clamp("b", "k"), "a".clamp("b", "k"));
    println!("{:?} {:?}", Size::L.clamp(Size::S, Size::M), Size::S.clamp(Size::M, Size::L));
    println!("{:?}", (3, 'x').clamp((1, 'a'), (3, 'b')));
    println!("{}", String::from("zz").clamp(String::from("a"), String::from("m")));
}
