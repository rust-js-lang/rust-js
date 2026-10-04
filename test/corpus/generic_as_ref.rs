// `AsRef` in generic code, as APIs take "anything that's text" or "anything
// that's a slice": a dictionary each caller gives, std's for a `String`, a
// `&str`, a `Vec` or an array, and the crate's own for its types.

fn shout<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_uppercase()
}

fn total<V: AsRef<[i32]>>(values: V) -> i32 {
    values.as_ref().iter().sum()
}

fn longest<'a, S: AsRef<str>>(words: &'a [S]) -> usize {
    words.iter().map(|w| w.as_ref().len()).max().unwrap_or(0)
}

struct Name {
    first: String,
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.first
    }
}

struct Scores(Vec<i32>);

impl AsRef<[i32]> for Scores {
    fn as_ref(&self) -> &[i32] {
        &self.0
    }
}

fn main() {
    let owned = String::from("tea");
    println!(
        "{} {} {} {}",
        shout("hi"),
        shout(&owned),
        shout(owned.clone()),
        shout(Name { first: "ada".to_string() })
    );
    println!(
        "{} {} {} {}",
        total([1, 2, 3]),
        total(vec![4, 5]),
        total(&[6][..]),
        total(Scores(vec![7, 8]))
    );
    println!("{} {}", longest(&["a", "abc"]), longest(&[String::from("four")]));
    let name = Name { first: "lin".to_string() };
    let direct: &str = name.as_ref();
    println!("{} {}", direct, Scores(vec![1, 2]).as_ref().len());
}
