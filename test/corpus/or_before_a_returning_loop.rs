// An `||` the program wrote whose branch leaves a block, before a loop
// that ends the block and never goes on, only returns: the `||` as
// written, `if (indent < 3 || (..)) { desc = ..; break; }`, then the loop,
// not an `else if` of the arm before (ADR 0805).
fn describe(keys: &[&str], indent: usize, out: &mut Vec<String>) -> String {
    let desc = match keys.len() {
        0 => "none".to_string(),
        n => 'desc: {
            if indent < 3 || (n == 2 && keys[0] == "children" && keys[1] == "type") {
                break 'desc format!("<{} … />", keys[0]);
            }
            let mut width = n * 10;
            loop {
                out.push(width.to_string());
                if width <= indent {
                    return out.join(",");
                }
                width /= 2;
            }
        }
    };
    desc + "!"
}

fn main() {
    let mut out = Vec::new();
    println!("{}", describe(&[], 0, &mut out));
    println!("{}", describe(&["a", "b"], 1, &mut out));
    println!("{}", describe(&["children", "type"], 5, &mut out));
    println!("{}", describe(&["children", "key"], 5, &mut out));
    println!("{}", describe(&["a", "b", "c"], 4, &mut out));
    println!("{out:?}");
}
