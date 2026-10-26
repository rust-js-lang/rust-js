// A loop's `continue`s the program wrote, as React's hydration diffs
// write them, before a `break`: each `if` of one where it's written, and a
// `for mut` binding of a loop deep in its function the loop's own (ADR
// 0684, amended).

fn described(props: &[(&str, u32)], mut room: i64) -> String {
    let mut content = String::new();
    for &(name, value) in props {
        if value == 0 {
            continue;
        }
        if name == "children" {
            // Ignored.
            continue;
        }
        let shown = value.to_string();
        room -= (name.len() + shown.len() + 2) as i64;
        if room < 0 {
            content += " ...";
            break;
        }
        content += &format!(" {name}={shown}");
    }
    format!("<{content}>")
}

fn renamed(kind: &str, names: &[&str], mut room: i64) -> String {
    match kind {
        "object" => {
            if names.is_empty() {
                return "{}".to_string();
            }
            let mut properties = String::new();
            for mut name in names.iter().map(|name| name.to_string()) {
                if name.starts_with('_') {
                    continue;
                }
                if name.len() > 3 {
                    name = format!("{name:?}");
                }
                room -= name.len() as i64;
                if room < 0 {
                    properties += if properties.is_empty() { "..." } else { ", ..." };
                    break;
                }
                properties += &((if properties.is_empty() { "" } else { "," }).to_owned() + &name);
            }
            "{".to_owned() + &properties + "}"
        }
        _ => kind.to_string(),
    }
}

fn main() {
    let props = [("a", 1), ("children", 2), ("b", 0), ("cc", 33), ("dddd", 4444)];
    for room in [0, 5, 12, 40] {
        println!("{}", described(&props, room));
    }
    let names = ["ab", "_x", "abcd", "e", "fghij"];
    for room in [0, 3, 9, 30] {
        println!("{} {}", renamed("object", &names, room), renamed("other", &names, room));
    }
    println!("{}", renamed("object", &[], 9));
}
