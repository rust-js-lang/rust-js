// A struct JS gives, an exported function's argument: each `Option` field
// read once where Rust reads it once, by each of `Option`'s combinators,
// however a getter answers it (#1908). A pattern's subject is #1924's. Each
// call's `gives` is native Rust's, which reads the field once: a getter
// that answers another value the second time, `$reads`, would change it
// where the JS read it twice. `scrubTo` is the SDK's, reduced.
//@ call: map [{"min": 0, "snap": {"$reads": [{"threshold": 1, "points": [2]}, null]}}]
//@ gives: 1
//@ logs: ["get call0.arg0.snap", "get call0.arg0.snap.threshold"]
//@ call: mapOr [{"min": 0, "snap": {"$reads": [{"threshold": 1, "points": [2]}, null]}}]
//@ gives: 1
//@ logs: ["get call1.arg0.snap", "get call1.arg0.snap.threshold"]
//@ call: mapOrElse [{"min": 0, "snap": {"$reads": [{"threshold": 1, "points": [2]}, null]}}]
//@ gives: 2
//@ logs: ["get call2.arg0.snap", "get call2.arg0.snap.threshold"]
//@ call: andThen [{"min": 0, "snap": {"$reads": [{"threshold": 1, "points": [2]}, null]}}]
//@ gives: 2
//@ logs: ["get call3.arg0.snap", "get call3.arg0.snap.points"]
//@ call: unwrapOr [{"min": 0, "step": {"$reads": [0.25, null]}}]
//@ gives: 0.25
//@ logs: ["get call4.arg0.step"]
//@ call: unwrapOrElse [{"min": 3, "step": {"$reads": [0.25, null]}}]
//@ gives: 0.25
//@ logs: ["get call5.arg0.step"]
//@ call: isSomeAnd [{"min": 0, "step": {"$reads": [0.75, 0.25]}}]
//@ gives: true
//@ logs: ["get call6.arg0.step"]
//@ call: isNoneOr [{"min": 0, "step": {"$reads": [0.25, null]}}]
//@ gives: false
//@ logs: ["get call7.arg0.step"]
//@ call: filter [{"min": 0, "step": {"$reads": [0.75, 0.25]}}]
//@ gives: 0.75
//@ logs: ["get call8.arg0.step"]
//@ call: okOr [{"min": 0, "step": {"$reads": [0.75, null]}}]
//@ gives: {"TAG":"Ok","_0":0.75}
//@ logs: ["get call9.arg0.step"]
//@ call: labelLen [{"min": 0, "label": {"$reads": ["four", null]}}]
//@ gives: 4
//@ logs: ["get call10.arg0.label"]
//@ call: scrubTo [1.3, {"min": 0, "step": 1, "snap": {"$reads": [{"threshold": 1, "points": [2]}, {"threshold": 0.01, "points": [2]}]}}]
//@ gives: {"value":2,"point":2}
//@ logs: ["get call11.arg1.snap", "get call11.arg1.snap.threshold", "get call11.arg1.snap", "get call11.arg1.snap.points"]

pub struct Snap {
    pub threshold: f64,
    pub points: Vec<f64>,
}

pub struct Range {
    pub min: f64,
    pub step: Option<f64>,
    pub snap: Option<Snap>,
    pub label: Option<String>,
}

pub fn map(r: &Range) -> Option<f64> {
    r.snap.as_ref().map(|s| s.threshold)
}

pub fn map_or(r: &Range) -> f64 {
    r.snap.as_ref().map_or(0.0, |s| s.threshold)
}

pub fn map_or_else(r: &Range) -> f64 {
    r.snap.as_ref().map_or_else(|| -1.0, |s| s.threshold * 2.0)
}

pub fn and_then(r: &Range) -> Option<f64> {
    r.snap.as_ref().and_then(|s| s.points.first().copied())
}

pub fn unwrap_or(r: &Range) -> f64 {
    r.step.unwrap_or(1.0)
}

pub fn unwrap_or_else(r: &Range) -> f64 {
    r.step.unwrap_or_else(|| r.min + 1.0)
}

pub fn is_some_and(r: &Range) -> bool {
    r.step.is_some_and(|s| s > 0.5)
}

pub fn is_none_or(r: &Range) -> bool {
    r.step.is_none_or(|s| s > 0.5)
}

pub fn filter(r: &Range) -> Option<f64> {
    r.step.filter(|s| *s > 0.5)
}

pub fn ok_or(r: &Range) -> Result<f64, &'static str> {
    r.step.ok_or("none")
}

pub fn label_len(r: &Range) -> usize {
    r.label.as_ref().map_or(0, |l| l.len())
}

pub struct Scrubbed {
    pub value: f64,
    pub point: Option<f64>,
}

/// The nearest snap point within the snap's threshold, else the value
/// rounded to the range's step.
pub fn scrub_to(value: f64, range: &Range) -> Scrubbed {
    let threshold = range.snap.as_ref().map_or(0.0, |snap| snap.threshold);
    let point = range.snap.as_ref().and_then(|snap| {
        let nearest = snap.points.iter().copied().min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))?;
        ((nearest - value).abs() <= threshold).then_some(nearest)
    });
    match point {
        Some(point) => Scrubbed { value: point, point: Some(point) },
        None => {
            let step = range.step.unwrap_or(1.0);
            Scrubbed { value: range.min + ((value - range.min) / step).round() * step, point: None }
        }
    }
}
