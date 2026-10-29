// A `return` of a `||` or `&&` chain in an arm of a `match` the function
// goes on past, whose `true` or `false` MIR shares with the function's
// end: the chain as written, `return tag === "tr" || tag === "style" ||
// ..;`, `return (scope && parent === "#document") || parent == null;`, not
// nested `if`s (rd95, React DOM's validateDOMNesting).
fn is_tag_valid(tag: &str, parent_tag: Option<&str>, implicit_root_scope: bool) -> bool {
    match parent_tag {
        Some("select") => {
            return tag == "hr" || tag == "option" || tag == "optgroup";
        }
        Some("tbody" | "thead") => {
            return tag == "tr" || tag == "style" || tag == "script" || tag == "template";
        }
        _ => {}
    }
    match tag {
        "h1" | "h2" | "h3" => {
            return parent_tag != Some("h1") && parent_tag != Some("h2") && parent_tag != Some("h3");
        }
        "html" => {
            return (implicit_root_scope && parent_tag == Some("#document")) || parent_tag.is_none();
        }
        _ => {}
    }
    true
}

fn main() {
    let parents = [None, Some("select"), Some("tbody"), Some("thead"), Some("h1"), Some("h3"), Some("#document"), Some("div")];
    for tag in ["hr", "option", "tr", "script", "template", "h1", "h2", "html", "p"] {
        for parent in parents {
            for scope in [false, true] {
                print!("{}", u8::from(is_tag_valid(tag, parent, scope)));
            }
        }
        println!(" {tag}");
    }
}
