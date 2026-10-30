// React's performWorkOnRoot: a loop's `if .. { ..; break; } else { .. }`
// then the `break` the program wrote after it, each where it's written,
// the `then`'s `break` and none at the `else`'s end, as upstream's `do {
// if (..) { ..; break; } else { ..; finishConcurrentRender(..); } break; }
// while (true)` (rd109's written jumps; the regression of uz2's kept
// `break`). The `else`'s own `continue`s and `break` stay as written.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    pub alternate: RefCell<Option<Rc<Fiber>>>,
}

pub struct FiberRoot {
    pub current: RefCell<Rc<Fiber>>,
    pub tag: u32,
}

type Lanes = u32;
type RootExitStatus = u32;
const NO_LANES: Lanes = 0;
const NO_LANE: Lanes = 0;
const ROOT_IN_PROGRESS: RootExitStatus = 0;
const ROOT_ERRORED: RootExitStatus = 2;
const ROOT_FATAL_ERRORED: RootExitStatus = 1;
const LEGACY_ROOT: u32 = 0;
const DISABLE_LEGACY_MODE: bool = false;
const ENABLE_COMPONENT_PERFORMANCE_TRACK: bool = true;

thread_local! {
    static ENABLE_PROFILER_TIMER: bool = true;
    static WORK_IN_PROGRESS_ROOT_IS_PRERENDERING: Cell<bool> = const { Cell::new(false) };
    static WORK_IN_PROGRESS_SUSPENDED_REASON: Cell<u32> = const { Cell::new(0) };
    static RENDER_START_TIME: Cell<f64> = const { Cell::new(0.0) };
    static STATUSES: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn next_status() -> u32 {
    STATUSES.with_borrow_mut(|s| s.pop().unwrap_or(5))
}
fn render_root_concurrent(_root: &Rc<FiberRoot>, _lanes: Lanes) -> RootExitStatus {
    next_status()
}
fn render_root_sync(_root: &Rc<FiberRoot>, _lanes: Lanes, _b: bool) -> RootExitStatus {
    next_status()
}
fn mark_root_suspended(_root: &Rc<FiberRoot>, lanes: Lanes, _l: Lanes, d: bool) {
    println!("suspended {lanes} {d}");
}
fn start_yield_timer(r: u32) {
    println!("yield {r}");
}
fn now() -> f64 {
    1.0
}
fn is_render_consistent_with_external_stores(_f: &Rc<Fiber>) -> bool {
    next_status() != 7
}
fn set_current_track_from_lanes(_l: Lanes) {}
fn log_inconsistent_render(_a: f64, _b: f64) {}
fn finalize_render(_l: Lanes, _t: f64) {}
fn get_lanes_to_retry_synchronously_on_error(_root: &Rc<FiberRoot>, l: Lanes) -> Lanes {
    l
}
fn recover_from_concurrent_error(_root: &Rc<FiberRoot>, _a: Lanes, _b: Lanes) -> RootExitStatus {
    next_status()
}
fn prepare_fresh_stack(_root: &Rc<FiberRoot>, _l: Lanes) {}
fn finish_concurrent_render(_root: &Rc<FiberRoot>, s: RootExitStatus, _f: &Rc<Fiber>, l: Lanes, _t: f64) {
    println!("finish {s} {l}");
}
fn ensure_root_is_scheduled(_root: &Rc<FiberRoot>) {
    println!("scheduled");
}

pub fn perform_work_on_root(root: &Rc<FiberRoot>, mut lanes: Lanes, should_time_slice: bool) {
    let mut exit_status: RootExitStatus = if should_time_slice {
        render_root_concurrent(root, lanes)
    } else {
        render_root_sync(root, lanes, true)
    };

    let mut render_was_concurrent = should_time_slice;

    loop {
        if exit_status == ROOT_IN_PROGRESS {
            // Render phase is still in progress.
            if WORK_IN_PROGRESS_ROOT_IS_PRERENDERING.get() && !should_time_slice {
                // We're in prerendering mode.
                let did_attempt_entire_tree = false;
                mark_root_suspended(root, lanes, NO_LANE, did_attempt_entire_tree);
            }
            if ENABLE_PROFILER_TIMER.with(|&enabled| enabled) && ENABLE_COMPONENT_PERFORMANCE_TRACK
            {
                // We're about to yield.
                start_yield_timer(WORK_IN_PROGRESS_SUSPENDED_REASON.get());
            }
            break;
        } else {
            let mut render_end_time = 0.0;
            if ENABLE_PROFILER_TIMER.with(|&enabled| enabled) && ENABLE_COMPONENT_PERFORMANCE_TRACK
            {
                render_end_time = now();
            }

            // The render completed.
            let finished_work = root.current.borrow().alternate.borrow().clone().unwrap();
            if render_was_concurrent && !is_render_consistent_with_external_stores(&finished_work) {
                if ENABLE_PROFILER_TIMER.with(|&enabled| enabled)
                    && ENABLE_COMPONENT_PERFORMANCE_TRACK
                {
                    set_current_track_from_lanes(lanes);
                    log_inconsistent_render(RENDER_START_TIME.get(), render_end_time);
                    finalize_render(lanes, render_end_time);
                }
                // A store was mutated in an interleaved event.
                exit_status = render_root_sync(root, lanes, false);
                render_was_concurrent = false;
                // Need to check the exit status again.
                continue;
            }

            // Check if something threw
            if (DISABLE_LEGACY_MODE || root.tag != LEGACY_ROOT) && exit_status == ROOT_ERRORED {
                let lanes_that_just_errored = lanes;
                let error_retry_lanes =
                    get_lanes_to_retry_synchronously_on_error(root, lanes_that_just_errored);
                if error_retry_lanes != NO_LANES {
                    lanes = error_retry_lanes;
                    exit_status =
                        recover_from_concurrent_error(root, lanes_that_just_errored, error_retry_lanes);
                    render_was_concurrent = false;
                    // Need to check the exit status again.
                    if exit_status != ROOT_ERRORED {
                        // The root did not error this time.
                        continue;
                    } else {
                        // The root errored yet again.
                        if ENABLE_PROFILER_TIMER.with(|&enabled| enabled)
                            && ENABLE_COMPONENT_PERFORMANCE_TRACK
                        {
                            render_end_time = now();
                        }
                    }
                }
            }
            if exit_status == ROOT_FATAL_ERRORED {
                prepare_fresh_stack(root, NO_LANES);
                // Since this is a fatal error.
                let did_attempt_entire_tree = true;
                mark_root_suspended(root, lanes, NO_LANE, did_attempt_entire_tree);
                break;
            }

            // We now have a consistent tree.
            finish_concurrent_render(root, exit_status, &finished_work, lanes, render_end_time);
        }
        break;
    }

    ensure_root_is_scheduled(root);
}

fn main() {
    for statuses in [vec![0], vec![1], vec![5], vec![3, 7, 5], vec![2, 2, 5]] {
        STATUSES.with_borrow_mut(|s| *s = statuses.into_iter().rev().collect());
        let fiber = Rc::new(Fiber { alternate: RefCell::new(None) });
        let alt = Rc::new(Fiber { alternate: RefCell::new(None) });
        *fiber.alternate.borrow_mut() = Some(alt);
        let root = Rc::new(FiberRoot { current: RefCell::new(fiber), tag: 1 });
        perform_work_on_root(&root, 4, true);
    }
}
