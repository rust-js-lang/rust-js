//! Where a compile's time goes (#2296): each phase's wall time, measured
//! only where `RUST_JS_PHASE_TIMES` names a file, to which one JSON line a
//! compile is appended, `{"input": "case.rs", "library": false, "phases":
//! [["rustc parse and expand", 41.2], ..]}`, in milliseconds, each phase
//! from where the last one ended, which the driver appends (`line`), and
//! `"forced"`, each of rustc's walks of every crate its lowering forced
//! (`forced`).
//! Without it, nothing is measured or written. Read by
//! `scripts/phase-times.ts`.

use std::sync::Mutex;
use std::time::Instant;

/// The variable that names the file a compile's times are appended to.
pub const VARIABLE: &str = "RUST_JS_PHASE_TIMES";

/// Where the last phase ended, each phase's time in milliseconds, and the
/// walks of every crate the compile forced.
type Timed = (Instant, Vec<(&'static str, f64)>, Vec<&'static str>);

/// Of a compile being timed, its `Timed`; `None` of one that isn't.
static TIMES: Mutex<Option<Timed>> = Mutex::new(None);

/// Starts timing, where `RUST_JS_PHASE_TIMES` is set.
pub fn start() {
    if std::env::var_os(VARIABLE).is_some() {
        *TIMES.lock().expect("the phase times") = Some((Instant::now(), Vec::new(), Vec::new()));
    }
}

/// The phase `phase` ended now, of a compile being timed.
pub fn mark(phase: &'static str) {
    if let Some((at, times, _)) = TIMES.lock().expect("the phase times").as_mut() {
        let now = Instant::now();
        times.push((phase, now.duration_since(*at).as_secs_f64() * 1000.0));
        *at = now;
    }
}

/// A query of rustc's that walks every crate a lowering forced, of a
/// compile being timed: `visible_parent_map`, which printing a type or a
/// path asks, 37 ms of each of the React port's compiles where its
/// lowering did (#2314); not where rustc's own warnings had already.
pub fn forced(query: &'static str) {
    if let Some((_, _, forced)) = TIMES.lock().expect("the phase times").as_mut()
        && !forced.contains(&query)
    {
        forced.push(query);
    }
}

/// The file `RUST_JS_PHASE_TIMES` names and the line of the compile of
/// `input`'s times to append to it, `library` of a library's build, where
/// it's timed: the driver writes it, as it writes the decisions' counts.
pub fn line(input: Option<&str>, library: bool) -> Option<(std::ffi::OsString, String)> {
    let path = std::env::var_os(VARIABLE)?;
    let (_, times, forced) = TIMES.lock().expect("the phase times").take()?;
    let phases: Vec<String> = (times.iter())
        .map(|(phase, ms)| format!("[{},{ms:.3}]", json_string(phase)))
        .collect();
    let line = format!(
        "{{\"input\":{},\"library\":{library},\"phases\":[{}],\"forced\":[{}]}}\n",
        input.map_or("null".to_string(), json_string),
        phases.join(","),
        forced
            .iter()
            .map(|query| json_string(query))
            .collect::<Vec<_>>()
            .join(",")
    );
    Some((path, line))
}

/// `text` as a JSON string.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string is JSON")
}
