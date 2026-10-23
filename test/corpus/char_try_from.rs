// char::try_from of a u32 and u8::try_from of a char, and their try_into():
// Ok in range, else CharTryFromError or TryFromCharError, shown as std's.
fn main() {
    let e = char::try_from(0xD800u32).unwrap_err();
    let f = u8::try_from('Ā').unwrap_err();
    let g = u8::try_from('😀').unwrap_err();
    println!("[{e}] [{e:?}] [{e:#?}]");
    println!("[{f}] [{f:?}] [{f:#?}] [{g}]");
    println!("{:?} {:?} {:?}", char::try_from(65u32), u8::try_from('A'), char::try_from(0x110000u32));
    let r: Result<char, _> = 97u32.try_into();
    let s: Result<u8, _> = 'z'.try_into();
    println!("{r:?} {s:?}");
}
