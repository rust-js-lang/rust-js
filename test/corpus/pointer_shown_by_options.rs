//@ native: wasm32
// A pointer's `{:p}` and `{:?}` by the `Formatter`'s options, as std's
// `pointer_fmt_inner` shows it: `LowerHex` of its address with `#` set, its
// sign, width, fill and alignment, and where `#` asks, zeros to its width,
// else to `usize`'s hex digits and `0x`, a field's in a `{:#?}` too; and
// written into a writer, each piece a call of the writer's (#2282). Of an
// address rustc and rust-js both know, an integer's or a dangling one's,
// the whole text; of an allocation's, which each places where it will (ADR
// 0562), what doesn't hang on where.
use std::fmt::Write;
use std::ptr::NonNull;

#[derive(Debug)]
struct Handle {
    id: u8,
    at: NonNull<u64>,
}

#[derive(Debug)]
struct Pair(*const u8, u16);

struct Counted(String, u32);

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.1 += 1;
        self.0.push_str(s);
        Ok(())
    }
}

fn counted(args: std::fmt::Arguments) -> String {
    let mut c = Counted(String::new(), 0);
    c.write_fmt(args).unwrap();
    format!("{} in {}", c.0, c.1)
}

fn main() {
    let p = 0x1234usize as *const u8;
    let null: *const u8 = std::ptr::null();
    let dangling = NonNull::<u64>::dangling();
    println!("{:p} {:?} {:p} {:?} {:?}", p, p, null, null, dangling);
    println!("[{:#?}] [{:#p}] [{:#?}] [{:#p}]", p, p, null, dangling);
    println!("[{:20p}] [{:<20p}] [{:^20?}] [{:*>20p}]", p, p, p, p);
    println!("[{:020p}] [{:+p}] [{:+?}] [{:#20p}] [{:#<20?}] [{:*<#20?}]", p, p, null, p, p, p);
    println!("[{:x?}] [{:#X?}] [{:3p}]", p, p, p);
    let width = 14;
    println!("[{0:>width$p}] [{1:#width$?}] [{1:-^2$p}]", p, null, 9);
    let handle = Handle { id: 7, at: dangling };
    println!("{:?}", handle);
    println!("{:#?}", handle);
    println!("{:12?}", handle);
    println!("{:#?}", Pair(p, 3));
    println!("{:#?}", (Some(dangling), [p, null]));
    // An allocation's address: its text is `{:#x}` of it, and `{:#?}`'s ten
    // `char`s.
    let held = 5u64;
    let at = &held as *const u64;
    let pretty = format!("{:#?}", at);
    println!(
        "{} {} {}",
        format!("{:p}", at) == format!("{:#x}", at as usize),
        pretty.len(),
        pretty.starts_with("0x")
    );
    // Written into a writer: `0x` and the digits two writes, each fill char
    // one.
    println!("{}", counted(format_args!("{:p}", p)));
    println!("{}", counted(format_args!("{:?}", null)));
    println!("{}", counted(format_args!("[{:8p}]", p)));
    println!("{}", counted(format_args!("{:#?}", p)));
    println!("{}", counted(format_args!("{:?}", handle)));
    println!("{}", counted(format_args!("{:#?}", Pair(p, 3))));
}
