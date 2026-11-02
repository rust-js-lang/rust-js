// A `#[rust_js::nullable]` field of an `Option` of an `Option`, WebIDL's
// optional nullable member, TypeScript's `sdpMid?: string | null`: its
// `None` not there, its `Some(None)` `null`, its `Some(Some(v))` `v`
// (#470, ADRs 0051 and 0810, amended, the user's decision). Made of each,
// of a constant, a variable, a parameter and another such field; read by
// `match`, `matches!`, `is_some()`, `is_none()`, `if let`, `flatten()`, a
// copy taken apart, given whole to a function, shown, compared and set
// again, against native Rust.
#[derive(Default)]
pub struct Candidate {
    pub candidate: Option<String>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub sdp_mid: Option<Option<String>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub line: Option<Option<u32>>,
}

fn kind(candidate: &Candidate) -> &'static str {
    match candidate.sdp_mid {
        None => "missing",
        Some(None) => "null",
        Some(Some(_)) => "value",
    }
}

fn tests(candidate: &Candidate) -> String {
    let mut out = String::new();
    if candidate.sdp_mid.is_some() {
        out.push('s');
    }
    if candidate.sdp_mid.is_none() {
        out.push('n');
    }
    if matches!(candidate.line, Some(None)) {
        out.push('0');
    }
    if let Some(Some(mid)) = &candidate.sdp_mid {
        out.push_str(mid);
    }
    if let Some(Some(line)) = candidate.line {
        out.push_str(&line.to_string());
    }
    out
}

fn flattened(candidate: &Candidate) -> String {
    let mid = candidate.sdp_mid.clone().flatten();
    let line = candidate.line.flatten();
    format!("{mid:?} {line:?}")
}

fn taken_apart(candidate: Candidate) -> String {
    let Candidate { sdp_mid, line, .. } = candidate;
    format!("{} {:?}", sdp_mid.flatten().unwrap_or_default(), line.flatten())
}

fn shown<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}")
}

fn made(at: u32, mid: Option<Option<String>>) -> Candidate {
    let line = if at == 0 {
        None
    } else if at == 1 {
        Some(None)
    } else {
        Some(Some(at))
    };
    Candidate {
        candidate: Some(format!("c{at}")),
        sdp_mid: mid,
        line,
    }
}

fn main() {
    let constants = [
        Candidate::default(),
        Candidate {
            sdp_mid: Some(None),
            line: Some(None),
            ..Default::default()
        },
        Candidate {
            sdp_mid: Some(Some("audio".to_string())),
            line: Some(Some(3)),
            ..Default::default()
        },
    ];
    for candidate in &constants {
        println!("{} {} {}", kind(candidate), tests(candidate), flattened(candidate));
        println!("{} {}", shown(&candidate.sdp_mid), shown(candidate.line));
    }
    let given = [None, Some(None), Some(Some("video".to_string()))];
    for (at, mid) in given.into_iter().enumerate() {
        let candidate = made(at as u32, mid);
        println!("{} {} {}", kind(&candidate), tests(&candidate), flattened(&candidate));
        let copied = Candidate {
            candidate: None,
            sdp_mid: candidate.sdp_mid.clone(),
            line: candidate.line,
        };
        println!("{} {}", copied.line == candidate.line, tests(&copied));
        println!("{}", taken_apart(candidate));
    }
    let mut changed = made(0, None);
    changed.line = Some(None);
    println!("{}", tests(&changed));
    changed.line = Some(Some(9));
    changed.sdp_mid = Some(None);
    println!("{} {}", tests(&changed), kind(&changed));
    changed.sdp_mid = None;
    println!("{} {}", tests(&changed), changed.candidate.is_some());
}
