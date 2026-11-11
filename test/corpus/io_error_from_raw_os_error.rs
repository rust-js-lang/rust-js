//@ library-differs
// `io::Error::from_raw_os_error(code)` is the OS's error of that number:
// its kind std's `decode_error_kind`, its message the platform's
// `strerror`, shown by `{}` and `{:?}` as std shows an OS error, its
// `raw_os_error()` the number again; one the platform doesn't know,
// `Uncategorized` and `strerror`'s words of a number it doesn't know
// (#1400, getrandom 0.1's `From<Error> for io::Error`). Each number here
// is the same on Linux and macOS.
use std::io;

fn main() {
    for code in [1, 2, 13, 17, 22, 0, 9999, -1] {
        let e = io::Error::from_raw_os_error(code);
        println!("{code}: {e} | {e:?} | {:?} {:?}", e.kind(), e.raw_os_error());
    }
    let e: Box<dyn std::error::Error + Send + Sync> = io::Error::from_raw_os_error(2).into();
    println!("{e}");
}
