// A string's iterator held in a variable and lent once, `cs.by_ref()`,
// read nowhere else: the string's own iterator, `cs.take(2)`, as one made
// where it's lent is, since it has no `return()` a lending keeps from being
// called and nothing else steps it (#1955). Of `chars()`, `char_indices()`
// and `nth`; and one stepped again after its lending, which stays lent,
// the rest of it read where the lending left it.
fn initials(name: &str) -> String {
    let mut cs = name.chars();
    cs.by_ref().take(2).collect()
}

fn after_first(s: &str) -> Vec<(usize, char)> {
    let mut cs = s.char_indices();
    cs.by_ref().skip(1).collect()
}

fn second(s: &str) -> Option<char> {
    let mut cs = s.chars();
    cs.nth(1)
}

fn head_and_rest(s: &str) -> (String, String) {
    let mut cs = s.chars();
    let head: String = cs.by_ref().take(2).collect();
    (head, cs.collect())
}

fn main() {
    println!("{}", initials("Ada Lovelace"));
    println!("{}", initials("é"));
    println!("{:?}", after_first("añb"));
    println!("{:?} {:?}", second("xyz"), second("x"));
    println!("{:?}", head_and_rest("héllo"));
}
