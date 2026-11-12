use dep::{Arbitrary, shrunk};

pub fn main() {
    println!("{:?}", shrunk(&100u32));
    println!("{:?}", shrunk(&'z'));
    let mut chars = 'a'.shrink();
    println!("{:?}", chars.next());
    println!("before the drop");
    drop(chars);
    println!("{:?}", 'q'.shrink().take(2).collect::<String>());
    for c in 'k'.shrink() {
        if c > 'd' {
            println!("found {c}");
            break;
        }
    }
    println!("{}", '\u{E001}'.shrink().count());
}
