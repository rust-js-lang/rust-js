// A `match` of constants whose arm binds what it matched, `symbol @
// (VIEW_TRANSITION | TRACING_MARKER)`, is still a `switch`, its binding the
// value the `switch` tests (rd351). React's getComponentNameFromType,
// whose upstream is `switch (type) { case REACT_FRAGMENT_TYPE: .. }`.
const FRAGMENT: u32 = 1;
const PROFILER: u32 = 2;
const VIEW_TRANSITION: u32 = 3;
const TRACING_MARKER: u32 = 4;

fn name_of(kind: Option<u32>, view_transitions: bool, tracing: bool) -> Option<&'static str> {
    match kind {
        Some(FRAGMENT) => return Some("Fragment"),
        Some(PROFILER) => return Some("Profiler"),
        Some(symbol @ (VIEW_TRANSITION | TRACING_MARKER)) => {
            if symbol == VIEW_TRANSITION && view_transitions {
                return Some("ViewTransition");
            }
            // Fall through
            if tracing {
                return Some("TracingMarker");
            }
        }
        _ => {}
    }
    None
}

fn main() {
    for kind in [None, Some(1), Some(2), Some(3), Some(4), Some(5)] {
        for (view, tracing) in [(false, false), (true, false), (false, true)] {
            println!("{kind:?} {view} {tracing} {:?}", name_of(kind, view, tracing));
        }
    }
}
