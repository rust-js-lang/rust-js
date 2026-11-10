//@ run-fail: range start index 3 out of range for slice of length 2
// `io::Read` and `io::BufRead` of a `&[u8]` (#1156): what's read is taken
// from its front, as std's impl takes it. A `read_exact` that can't fill
// its buffer empties the slice first; a `read_to_string` of what isn't
// UTF-8 takes nothing; `consume` past its end panics as slicing does.
use std::io::{BufRead, Read};

fn main() {
    let data = [1u8, 2, 3, 4, 5];
    let mut src: &[u8] = &data;
    let mut two = [0u8; 2];
    println!("{:?} {two:?} {src:?}", src.read(&mut two).unwrap());
    let mut one = [0u8; 1];
    println!("{:?} {one:?} {src:?}", src.read(&mut one).unwrap());
    let mut big = [9u8; 4];
    println!("{:?} {big:?} {src:?}", src.read(&mut big).unwrap());
    println!("{:?} {src:?}", src.read(&mut big).unwrap());

    let mut short: &[u8] = &[7, 8];
    let mut four = [0u8; 4];
    let e = short.read_exact(&mut four).unwrap_err();
    println!("{:?} {e} {e:?} {four:?} {short:?}", e.kind());
    let mut exact: &[u8] = &[7, 8, 9];
    exact.read_exact(&mut two).unwrap();
    println!("{two:?} {exact:?}");
    exact.read_exact(&mut []).unwrap();
    println!("{exact:?}");

    let mut rest: &[u8] = b"tail";
    let mut v = vec![b'>'];
    println!("{:?} {v:?} {rest:?}", rest.read_to_end(&mut v).unwrap());

    let mut text: &[u8] = "héllo".as_bytes();
    let mut s = String::from("say ");
    println!("{:?} {s} {text:?}", text.read_to_string(&mut s).unwrap());
    let mut bad: &[u8] = &[b'o', b'k', 0xff];
    let mut kept = String::from("kept");
    let e = bad.read_to_string(&mut kept).unwrap_err();
    println!("{:?} {e} {kept} {bad:?}", e.kind());

    let mut buffered: &[u8] = b"abcdef";
    let filled = buffered.fill_buf().unwrap();
    println!("{filled:?}");
    buffered.consume(4);
    println!("{buffered:?}");
    buffered.consume(3);
}
