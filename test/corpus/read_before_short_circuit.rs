// A borrowed `&str`'s `to_string()`, joined to an `if` whose `&&` and `||`
// test before the one branch that drops: every way through the tests joins
// the prefix, not just the one past the first test, as React's
// `mapIntoArray` keys a mapped child (ADR 0451). A `String` dropped in the
// branch, and a `Drop` type.

fn escape(key: &str) -> String {
    key.replace('=', "=0")
}

fn text(key: Option<&str>) -> Option<String> {
    key.map(|k| format!("{k}"))
}

fn key_of(prefix: &str, mapped: Option<&str>, child: Option<&str>, child_key: String) -> String {
    prefix.to_string()
        + &(if mapped.is_some() && (child.is_none() || child != mapped) {
            escape(&text(mapped).unwrap()) + "/"
        } else {
            String::new()
        })
        + &child_key
}

// Its branch a statement, and a drop JS sees before the test: the prefix
// still read where it's joined.
fn logged(prefix: &str, mapped: Option<&str>) -> String {
    let first = Loud(0);
    drop(first);
    prefix.to_string()
        + &(if mapped.is_some() && mapped != Some("") {
            println!("mapped");
            escape(&text(mapped).unwrap()) + "/"
        } else {
            String::new()
        })
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn tagged(prefix: &str, mapped: Option<u32>, child: Option<u32>) -> String {
    prefix.to_string()
        + &(if mapped.is_some() && (child.is_none() || child != mapped) {
            let loud = Loud(mapped.unwrap());
            format!("{}/", loud.0)
        } else {
            String::new()
        })
        + "."
}

fn main() {
    println!("{}", key_of(".$", None, None, String::from("0")));
    println!("{}", key_of(".$", Some("a=b"), None, String::from("1")));
    println!("{}", key_of(".$", Some("a"), Some("a"), String::from("2")));
    println!("{}", key_of(".$", Some("a"), Some("b"), String::from("3")));
    println!("{}", key_of("", None, Some("b"), String::from("4")));
    println!("{}", logged("$", None));
    println!("{}", logged("$", Some("")));
    println!("{}", logged("$", Some("k")));
    println!("{}", tagged("$", None, None));
    println!("{}", tagged("$", Some(1), None));
    println!("{}", tagged("$", Some(2), Some(2)));
    println!("{}", tagged("$", Some(3), Some(4)));
}
