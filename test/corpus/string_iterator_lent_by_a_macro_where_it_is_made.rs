// A macro lends what it's given, `$it.by_ref()`, so a variable's iterator
// goes on from where the macro stopped. Given a string's iterator made
// where it's lent, nothing else holds it, so it's the string's own
// iterator, not one lent (#1186).
macro_rules! head {
    ($it:expr, $n:expr) => {
        $it.by_ref().take($n).collect::<String>()
    };
}

macro_rules! indices_after {
    ($it:expr, $n:expr) => {
        $it.by_ref().skip($n).map(|(i, _)| i).collect::<Vec<_>>()
    };
}

fn main() {
    let text = "héllo wörld";
    println!("{}", head!(text.chars(), 3));
    println!("{:?}", indices_after!(text.char_indices(), 8));

    let mut rest = text.chars();
    let first = head!(rest, 5);
    println!("{first} then {:?}", rest.collect::<String>());
}
