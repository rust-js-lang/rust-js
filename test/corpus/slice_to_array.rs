// A slice into an array, `<&[u8; 3]>::try_from(&bytes[..3])`, as chrono
// reads a date's digits: `Ok` of its length, else a `TryFromSliceError`.
use std::convert::TryFrom;

fn main() {
    let bytes = [1u8, 2, 3, 4, 5];
    let fixed = <&[u8; 3]>::try_from(&bytes[..3]).unwrap();
    let owned = <[u8; 2]>::try_from(&bytes[1..3]).unwrap();
    let wrong = <&[u8; 4]>::try_from(&bytes[..3]);
    println!("{fixed:?} {owned:?} {wrong:?} {}", wrong.is_err());
    match <[u8; 2]>::try_from(&bytes[..]) {
        Ok(two) => println!("{two:?}"),
        Err(e) => println!("{e}"),
    }
    let words = vec![String::from("a"), String::from("b")];
    let pair: &[String; 2] = words.as_slice().try_into().unwrap();
    println!("{} {}", pair[0], pair[1]);
}
