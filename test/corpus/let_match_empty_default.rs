// A `let` of a tuple's `match` whose arms' failures share a last `_ =>
// Vec::new()`, one of them a guard's, gives the empty array first, `let
// entries = [];`, as it does a constant, and no labeled block, as the
// playground's `prepare` picks its entries (#556). The array is new each
// time, and nothing holds it but the binding.
fn entries(test: bool, tests: Option<(&str, &[String])>, main: bool, root: &str, sources: &mut Vec<String>) -> Vec<String> {
    let entries = match (test, tests) {
        (true, Some((module, files))) => {
            sources.push(module.to_string());
            files.to_vec()
        }
        (false, _) if main => vec![root.to_string()],
        _ => Vec::new(),
    };
    sources.push(root.to_string());
    entries
}

fn main() {
    let files = vec!["a.test.js".to_string(), "b.test.js".to_string()];
    let mut sources = Vec::new();
    for test in [true, false] {
        for tests in [Some(("test", &files[..])), None] {
            for main in [true, false] {
                let mut e = entries(test, tests, main, "lib.js", &mut sources);
                e.push("end".to_string());
                println!("{test} {} {main}: {e:?}", tests.is_some());
            }
        }
    }
    println!("{sources:?}");
}
