// A reference made before a branch and read only in one branch is the
// place it refers to there: a variable another branch writes isn't held
// for it, as nothing reads it past the write. Its `if let Some(found) =
// &mut found` is `found` itself, and the `else`'s `found = ..` assigns it.
// So is one the program names, `let r = &mut found; match r { .. }`, whose
// `None` arm writes `found`. And a reborrow read past a reassignment in one
// branch, and in another, is of what the reference was in each (ADR 0099).

fn collect(values: &[u32]) -> Option<Vec<u32>> {
    let mut found: Option<Vec<u32>> = None;
    for &value in values {
        if value > 1 {
            if let Some(found) = &mut found {
                found.push(value);
            } else {
                found = Some(vec![value]);
            }
        }
    }
    found
}

fn last_word(words: &[&str]) -> String {
    let mut seen: Option<String> = None;
    for word in words {
        match &mut seen {
            Some(seen) if seen.len() < 8 => seen.push_str(word),
            _ => seen = Some(word.to_string()),
        }
    }
    seen.unwrap_or_default()
}

fn named(values: &[u32]) -> Option<Vec<u32>> {
    let mut found: Option<Vec<u32>> = None;
    for &v in values {
        let r = &mut found;
        match r {
            Some(f) => f.push(v),
            None => found = Some(vec![v * 10]),
        }
    }
    found
}

fn reborrowed(c: bool) -> String {
    let mut p = vec![1];
    let mut q = vec![2];
    let mut x = &mut p;
    let r = &mut *x;
    if c {
        x = &mut q;
        r.push(3);
        x.push(4);
    } else {
        r.push(5);
    }
    format!("{p:?} {q:?}")
}

fn rematched(c: bool) -> String {
    let mut p = Some(vec![1]);
    let mut q = Some(vec![2]);
    let mut x = &mut p;
    match x {
        Some(v) if c => {
            x = &mut q;
            v.push(3);
            x.as_mut().unwrap().push(4);
        }
        Some(v) => v.push(5),
        None => {}
    }
    format!("{p:?} {q:?}")
}

fn main() {
    println!("{:?}", collect(&[1, 2, 3, 0, 4]));
    println!("{:?}", collect(&[0, 1]));
    println!("{}", last_word(&["ab", "cd", "efgh", "ij"]));
    println!("{:?}", named(&[6, 7]));
    println!("{}", reborrowed(true));
    println!("{}", reborrowed(false));
    println!("{}", rematched(true));
    println!("{}", rematched(false));
}
