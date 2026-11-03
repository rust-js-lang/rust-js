// A plain `if let`'s binding of a field by the field's own name, its test
// of that field, is the field read again where the statement after it
// reads it first, nothing run between, as React's
// ReactFlightClient writes `if (info.key != null) { add(info.key) }`
// (rd416, ADR 0866, amended): `name` and `props` here. Another name keeps
// its `const`, as does a binding read after something runs, one read
// twice, and a clone held before its test, `key`.

#[derive(Debug)]
struct Info {
    key: Option<String>,
    name: Option<&'static str>,
    props: Option<&'static str>,
    owner: Option<u32>,
    depth: Option<u32>,
}

fn add(name: &str, value: Option<String>, out: &mut Vec<String>) {
    match value {
        Some(value) => out.push(format!("{name}={value}")),
        None => out.push(name.to_owned()),
    }
}

fn bump(info: &mut Info) -> u32 {
    info.depth = Some(info.depth.unwrap_or(0) + 10);
    1
}

fn properties_of(info: &mut Info) -> Vec<String> {
    let mut properties = Vec::new();
    if let Some(key) = info.key.clone() {
        add("key", Some(key), &mut properties);
    }
    if let Some(name) = info.name {
        add("name", Some(name.to_owned()), &mut properties);
    }
    if let Some(props) = info.props {
        add(props, None, &mut properties);
    }
    if let Some(id) = info.owner {
        add("owner", Some(id.to_string()), &mut properties);
    }
    if let Some(depth) = info.depth {
        let more = bump(info);
        add("depth", Some((depth + more).to_string()), &mut properties);
    }
    if let Some(owner) = info.owner {
        add("twice", Some((owner + owner).to_string()), &mut properties);
    }
    properties
}

fn main() {
    let mut info = Info {
        key: Some("k".to_owned()),
        name: Some("n"),
        props: Some("props"),
        owner: Some(3),
        depth: Some(2),
    };
    println!("{:?}", properties_of(&mut info));
    println!("{info:?}");
    let mut empty = Info {
        key: None,
        name: None,
        props: None,
        owner: None,
        depth: None,
    };
    println!("{:?}", properties_of(&mut empty));
}
