use dep::{Buf, Items, Plain};

pub fn main() {
    let buf = Buf { bytes: [0; 3] };
    println!("{} {buf} {}", Plain(7), Items(vec!['a', 'b']));
    let text = format!("{}|{}", Buf { bytes: [1; 0] }, Items::<u8>(vec![]));
    println!("{text} {}", text.len());
}
