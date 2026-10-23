// Format options that reach what shows a value: `{:.2?}` of a `Duration`,
// rounded as std's is, a tie to even; `{:#?}` and a width of std's
// errors; and an alignment, a fill or `0` without a width, which a `fmt`
// asks of its `Formatter`.
use std::fmt;
use std::time::Duration;

struct Asked;

impl fmt::Display for Asked {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let align = match f.align() {
            None => "none",
            Some(fmt::Alignment::Left) => "left",
            Some(fmt::Alignment::Right) => "right",
            Some(fmt::Alignment::Center) => "center",
        };
        write!(f, "{align} {:?} {}", f.fill(), f.sign_aware_zero_pad())
    }
}

fn main() {
    let d = Duration::from_millis(1500);
    println!("{d:.2?} {d:.0?} {:.1?} {:.3?}", Duration::from_nanos(1_000_001), Duration::from_nanos(1_000_500));
    println!("{:#?}", "x".parse::<u8>().unwrap_err());
    println!("{:#?}", u8::try_from(-1i32).unwrap_err());
    let bad = std::str::from_utf8(&[b'a', 0xff]).unwrap_err();
    println!("{bad:3?}");
    println!("{bad:#?}");
    println!("{:<} {:^} {:*<} {:0}", Asked, Asked, Asked, Asked);
}
