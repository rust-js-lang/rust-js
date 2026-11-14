// A consumer's calls of a library's `From`s into its untagged enum: each
// the library's `from`, called, but the one its contract says gives its
// argument, and generic code's of either, the library's and the crate's.
use dep::{Detail, detail, show};

fn own<T: Into<Detail>>(value: T) -> Detail {
    value.into()
}

pub fn main() {
    let text = detail("hello");
    let three: Detail = 3u32.into();
    let five = Detail::from(5u32);
    println!("{}:{}:{}", show(&text), show(&three), show(&five));
    let kept: Detail = String::from("kept").into();
    let parts: Vec<Detail> = [-1i64, 1].into_iter().map(Detail::from).collect();
    println!(
        "{} {} {} {} {} {}",
        show(&kept),
        show(&parts[0]),
        show(&parts[1]),
        show(&own(String::from("own"))),
        show(&own(9u32)),
        show(&own("str"))
    );
    println!("{}", Detail::from("x") == Detail::Text("x".to_owned()));
}
