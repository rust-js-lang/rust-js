use dep::Ring;

pub fn main() {
    let names = [String::from("x"), String::from("y"), String::from("z")];
    let turned = dep::rotated(&names);
    println!("{} {}", dep::joined(turned, " "), dep::joined(names, "+"));
    let mut ring = Ring {
        items: [String::from("m"), String::from("n")],
    };
    let turned = ring.turned();
    ring.items[0].push('!');
    println!("{} {:?} {:?}", turned.shown(), turned.items, ring.items);
    let words = Ring {
        items: [String::from("p"), String::from("q"), String::from("r"), String::from("s")],
    };
    println!("{}", words.turned().turned().shown());
    println!("{}", dep::report());
}
