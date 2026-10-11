// A local of a function pointer type whose parameter borrows, `for<'a>
// fn(Props<'a>)`: what it holds is read as any value is.
struct Props<'a> {
    text: &'a str,
}

fn length(props: Props) -> usize {
    props.text.len()
}

fn main() {
    let measure: fn(Props) -> usize = length;
    println!("{}", measure(Props { text: "abc" }));
}
