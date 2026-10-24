// An `isize` counter from 0 below `len() as isize` is a JS index as it is,
// and stays at most the length past its loop, so `s != len() as isize`
// puts it below it (ADR 0709): React's `describeNativeComponentFrame` as
// ported, its indices `isize` from the start, read `sampleLines[s]` as
// upstream reads them.
fn frame(sample_stack: &str, control_stack: &str) -> String {
    let sample_lines: Vec<&str> = sample_stack.split('\n').collect();
    let control_lines: Vec<&str> = control_stack.split('\n').collect();
    let mut s: isize = 0;
    let mut c: isize = 0;
    while s < sample_lines.len() as isize && !sample_lines[s as usize].contains("Root") {
        s += 1;
    }
    while c < control_lines.len() as isize && !control_lines[c as usize].contains("Root") {
        c += 1;
    }
    if s == sample_lines.len() as isize || c == control_lines.len() as isize {
        s = sample_lines.len() as isize - 1;
        c = control_lines.len() as isize - 1;
        while s >= 1 && c >= 0 && sample_lines[s as usize] != control_lines[c as usize] {
            c -= 1;
        }
    }
    while s >= 1 && c >= 0 {
        if sample_lines[s as usize] != control_lines[c as usize] {
            if s != 1 || c != 1 {
                loop {
                    s -= 1;
                    c -= 1;
                    if c < 0 || sample_lines[s as usize] != control_lines[c as usize] {
                        return sample_lines[s as usize].replacen(" at new ", " at ", 1);
                    }
                    if !(s >= 1 && c >= 0) {
                        break;
                    }
                }
            }
            break;
        }
        s -= 1;
        c -= 1;
    }
    String::new()
}

// A counter past its loop that may be the length is still checked where
// it's read: `s` is the length when nothing matched.
fn found<'a>(lines: &[&'a str]) -> &'a str {
    let mut s: isize = 0;
    while s < lines.len() as isize && !lines[s as usize].contains("Root") {
        s += 1;
    }
    lines[s as usize]
}

fn main() {
    let sample = "Error\n    at new Button (app.js:3)\n    at Root (react.js:9)\n    at main";
    let control = "Error\n    at fake (react.js:5)\n    at Root (react.js:9)\n    at main";
    println!("{:?}", frame(sample, control));
    // No common root: the stacks matched from their ends.
    let sample = "Error\n    at Inner (a.js:1)\n    at Outer (a.js:2)\n    at run";
    let control = "Error\n    at fake (b.js:1)\n    at run";
    println!("{:?}", frame(sample, control));
    println!("{:?}", frame("Error", "Error"));
    println!("{:?}", frame("", "Error\nRoot"));
    println!("{:?}", frame("a\nb\nRoot", "Root"));
    println!("{}", found(&["a", "Root"]));
    let missing = std::panic::catch_unwind(|| found(&["a", "b"]).len());
    println!("{}", missing.is_err());
}
