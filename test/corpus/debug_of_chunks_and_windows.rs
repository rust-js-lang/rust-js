// `{:?}` of a slice's `Chunks` and `Windows` is std's derived `Debug` of its
// struct, `Chunks { v: [5], chunk_size: 2 }`: the slice it has left, as
// stepping from either end leaves it, once it's done too (a `Windows`
// keeps what's shorter than its window), and the size it cuts by, through
// generic code, `{:#?}`, `{:x?}` and an `Option` alike (#2252).
use std::fmt::Debug;

fn show<T: Debug>(t: &T) -> String {
    format!("{t:?}")
}

fn show_hex<T: Debug>(t: &T) -> String {
    format!("{t:x?}")
}

fn main() {
    let v = [1, 2, 3, 4, 5];
    let fresh = v.chunks(2);
    println!("{fresh:?}");
    let mut stepped = v.chunks(2);
    stepped.next();
    println!("{stepped:?} {}", show(&stepped));
    stepped.next_back();
    println!("{stepped:?}");
    stepped.next();
    println!("{stepped:?} {:?}", stepped.next());
    println!("{:#?}", v.chunks(3));
    println!("{:x?}", [10, 11, 12].chunks(2));
    println!("{:x?} {:#x?}", [255u8; 3].chunks(16), [1u8; 2].windows(17));
    println!("{}", show_hex(&[255u8; 3].chunks(16)));
    let mut w = v.windows(3);
    println!("{w:?}");
    w.next();
    w.next_back();
    println!("{w:?}");
    w.next();
    println!("{w:?} {:?}", w.next());
    println!("{:?}", [1, 2].windows(4));
    println!("{:?}", Some([7, 8, 9].windows(2)));
    println!("{:#?}", [7, 8, 9].windows(2));
    let empty: &[u8] = &[];
    println!("{:?} {:?}", empty.chunks(2), empty.windows(1));
    let words = ["a", "bc", "d"];
    let mut pairs = words.windows(2);
    pairs.next();
    println!("{pairs:?}");
}
