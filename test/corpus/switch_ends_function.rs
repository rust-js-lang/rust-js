// A `match` that ends a function of no value is a `switch` whose cases
// each end in `break;`, `case HostText: if (flags) { .. } break;`, as
// React's recursivelyInsertNewFiber writes it, not the `return;` MIR
// makes of the way to the function's end, in each case and each `if` that
// ends one (ADR 0905, amended).
const FUNCTION_COMPONENT: u8 = 0;
const FORWARD_REF: u8 = 11;
const HOST_COMPONENT: u8 = 5;
const HOST_TEXT: u8 = 6;
const HOST_PORTAL: u8 = 4;
const OFFSCREEN_COMPONENT: u8 = 22;
const HOST_HOISTABLE: u8 = 26;
const HOST_SINGLETON: u8 = 27;

fn visit(tag: u8, flags: u8, resources: bool, singletons: bool, log: &mut Vec<u8>) {
    match tag {
        FUNCTION_COMPONENT | FORWARD_REF => {
            log.push(1);
            if flags != 0 {
                log.push(2);
            }
        }
        HOST_HOISTABLE if resources => {
            log.push(3);
        }
        // Fall through
        HOST_HOISTABLE | HOST_SINGLETON if singletons => {
            log.push(4);
            if flags != 0 {
                log.push(5);
            }
        }
        // Fall through
        HOST_HOISTABLE | HOST_SINGLETON | HOST_COMPONENT => {
            log.push(6);
        }
        HOST_TEXT => {
            if flags != 0 {
                log.push(7);
            }
        }
        HOST_PORTAL => {}
        OFFSCREEN_COMPONENT => {
            if flags == 0 {
                log.push(8);
                return;
            }
            log.push(9);
        }
        _ => {
            for i in 0..flags {
                if i == 2 {
                    return;
                }
                log.push(10 + i);
            }
        }
    }
}

const TRACING: bool = true;
const LEGACY_HIDDEN: bool = false;

fn bailout(tag: u8, log: &mut Vec<u8>) -> Option<u8> {
    log.push(50);
    Some(tag)
}

// A `match` followed by the function's one `return`, guarded arms that
// fall to nothing last: each case's `break;` before that `return`, React's
// attemptEarlyBailoutIfNoScheduledUpdate, not MIR's copy of it in each.
fn attempt(tag: u8, lanes: u8, log: &mut Vec<u8>) -> Option<u8> {
    match tag {
        HOST_COMPONENT => {
            let next = lanes + 1;
            log.push(next);
            if lanes > 2 {
                log.push(lanes);
            }
        }
        HOST_TEXT | HOST_PORTAL => log.push(21),
        OFFSCREEN_COMPONENT => {
            if lanes == 0 {
                return None;
            }
            log.push(22);
        }
        FORWARD_REF => {
            log.push(23);
            return bailout(0, log);
        }
        HOST_HOISTABLE if TRACING => {
            log.push(24);
            return bailout(tag, log);
        }
        // Fallthrough
        HOST_HOISTABLE | HOST_SINGLETON if LEGACY_HIDDEN => {
            log.push(25);
            return Some(lanes);
        }
        _ => {}
    }
    bailout(tag, log)
}

fn main() {
    let mut log = Vec::new();
    for tag in [0, 11, 5, 6, 4, 22, 26, 27, 99] {
        for flags in 0..4 {
            for (resources, singletons) in [(false, false), (true, false), (false, true)] {
                visit(tag, flags, resources, singletons, &mut log);
                log.push(0);
            }
        }
    }
    println!("{log:?}");
    let mut log = Vec::new();
    for tag in [0, 11, 5, 6, 4, 22, 26, 27, 99] {
        for lanes in 0..4 {
            let got = attempt(tag, lanes, &mut log);
            log.push(got.map_or(99, |g| g + 100));
        }
    }
    println!("{log:?}");
}
