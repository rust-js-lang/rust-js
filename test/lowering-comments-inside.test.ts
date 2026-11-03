// Where the compiler writes a comment inside an expression (ADR 0630): a
// block comment after the code it's after, where JS has that code on a line
// of more. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, lib: await import(join(dir, "lib.js")) };
}

// React's ReactFiberHotReloading: `if (payload._status === 1 /* Resolved;
// see ReactLazy */) {`. Its port's call, an argument a line as rustfmt
// writes it, `1, /* Resolved; see ReactLazy */`: the comment after its
// argument, inside the test, as upstream has it, not in the `if`'s block
// (ADR 0630, amended); a line comment there is the line's, after it.
test("a block comment after an argument's comma is after the argument", async () => {
  const { js, lib } = await compiled("comment-after-an-argument", `use js::Unknown;

pub fn resolved(payload: Option<&'static Unknown>, count: &mut u32) -> bool {
    if js::strict_equals(
        js::get(payload, "_status"),
        1, /* Resolved; see ReactLazy */
    ) {
        *count += 1;
        return true;
    }
    false
}

pub fn pending(payload: Option<&'static Unknown>) -> bool {
    js::strict_equals(
        js::get(payload, "_status"),
        0, // Pending
    )
}
`);
  expect(js).toContain("  if (payload._status === 1 /* Resolved; see ReactLazy */) {\n    count.value = ");
  expect(js).toContain("  return payload._status === 0; // Pending\n");
  const count = { value: 0 };
  expect([lib.resolved({ _status: 1 } as never, count as never), lib.resolved({ _status: 0 } as never, count as never), count.value]).toEqual([true, false, 1]);
  expect(lib.pending({ _status: 0 } as never)).toBe(true);
});

// React's isSameOriginFrame, `try { // Reading href can throw. return
// typeof iframe.contentWindow.location.href === 'string'; } catch (err) {
// return false; }`: its port's comment, first in the `catch_unwind`
// closure of one value, `unwrap_or(false)`'s, was before the `try` (ADR
// 0444, amended).
test("a comment before a catch_unwind closure's value is in the try", async () => {
  const { js, lib } = await compiled("comment-in-a-closures-value", `use std::panic::{AssertUnwindSafe, catch_unwind};

use js::{Kind, Unknown};

pub fn is_text_href(frame: Option<&'static Unknown>) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        // Reading a cross-origin frame's href throws.
        matches!(js::classify(js::get(js::get(frame, "location"), "href")), Kind::String(_))
    }))
    .unwrap_or(false)
}
`);
  expect(js).toContain(`  try {
    // Reading a cross-origin frame's href throws.
    return typeof frame.location.href === "string";
  } catch {
    return false;
  }`);
  expect([lib.isTextHref({ location: { href: "a" } } as never), lib.isTextHref(undefined as never)]).toEqual([true, false]);
});

// React DOM's FizzInstructionSetShared revealCompletedBoundariesWithViewTransitions:
// `try { if (..) { ..; return; } // Fall through to reveal. } catch (x) { //
// Fall through to reveal. }`, a flag out of `catch_unwind` returned from the
// function. Its port's comment before the closure's `false`, which makes no
// JS, is at the `try`'s end, and the one above the folded `if let Ok(true)`
// in the `catch`, that test's other way, where the closure threw; both were
// above the `try`, which was placed at that `if` (rd634).
test("a comment before a flagged catch_unwind's false is at its try's end, one above its test in the catch", async () => {
  const { js, lib } = await compiled("comments-of-a-flagged-try", `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn reveal(on: bool, log: &mut Vec<u32>) {
    let started = catch_unwind(AssertUnwindSafe(|| {
        if on {
            log.push(1);
            return true;
        }
        // Fall through to reveal.
        false
    }));
    // Fall through, as it threw.
    if let Ok(true) = started {
        return;
    }
    log.push(2);
}
`);
  expect(js).toContain(`  try {
    if (on) {
      log.push(1);
      return;
    }
    // Fall through to reveal.
  } catch {
    // Fall through, as it threw.
  }
  log.push(2);`);
  const log: number[] = [];
  lib.reveal(true as never, log as never);
  lib.reveal(false as never, log as never);
  expect(log).toEqual([1, 2]);
});

// A comment above the `if r.is_err()` a `try` folds, whose arm is the
// `Err`'s, is the `catch`'s, before what it does, its payload's binding
// too; it was lost (#489). Not one above a `match`'s `Ok` arm, whose
// `if` MIR makes of the `Err` first: that arm's, where it is, and never the
// `catch`'s, of an arm of nothing too.
test("a comment above a folded is_err test is first in its catch", async () => {
  const { js, lib } = await compiled("comment-above-an-err-test", `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn errored(on: bool, log: &mut Vec<u32>) {
    let started = catch_unwind(AssertUnwindSafe(|| {
        if on {
            panic!("off");
        }
        log.push(1);
    }));
    // What threw is logged.
    if started.is_err() {
        log.push(9);
        log.push(10);
    }
    log.push(2);
}

pub fn returned(on: bool) -> u32 {
    let made = catch_unwind(AssertUnwindSafe(|| {
        if on {
            panic!("off");
        }
        5
    }));
    // Of the Ok arm, not the catch's.
    if let Ok(n) = made {
        return n;
    }
    0
}

pub fn measured(on: bool, log: &mut Vec<usize>) {
    let started = catch_unwind(AssertUnwindSafe(|| {
        if on {
            panic!("off");
        }
        log.push(1);
    }));
    // What threw is measured.
    if let Err(e) = started {
        let said = e.downcast_ref::<&str>().map_or(0, |s| s.len());
        log.push(said);
    }
    log.push(2);
}

pub fn matched(on: bool, log: &mut Vec<u32>) -> u32 {
    let made = catch_unwind(AssertUnwindSafe(|| {
        if on {
            panic!("off");
        }
        5
    }));
    match made {
        // The value.
        Ok(n) => log.push(n),
        // What threw.
        Err(_) => return 3,
    }
    0
}

pub fn ignored(on: bool, log: &mut Vec<u32>) {
    let made = catch_unwind(AssertUnwindSafe(|| {
        if on {
            panic!("off");
        }
        log.push(1);
    }));
    match made {
        // Fine.
        Ok(()) => {}
        Err(_) => log.push(3),
    }
    log.push(2);
}
`);
  expect(js).toContain(`  } catch {
    return 0;
  }`);
  expect(js).toContain(`  } catch {
    // What threw is logged.
    log.push(9);
    log.push(10);
  }
  log.push(2);`);
  expect(js).toContain(`  } catch (thrown) {
    // What threw is measured.
    const e = $unwound(thrown);`);
  expect(js).toContain(`  } catch {
    // What threw.
    return 3;
  }
  // The value.
  log.push(n);`);
  expect(js).toContain(`  } catch {
    log.push(3);
  }`);
  const log: number[] = [];
  lib.errored(false as never, log as never);
  lib.errored(true as never, log as never);
  expect(log).toEqual([1, 2, 9, 10, 2]);
  expect([lib.returned(false as never), lib.returned(true as never)]).toEqual([5, 0]);
  const measured: number[] = [];
  lib.measured(false as never, measured as never);
  lib.measured(true as never, measured as never);
  expect(measured).toEqual([1, 2, 3, 2]);
  const matched: number[] = [];
  expect([lib.matched(false as never, matched as never), lib.matched(true as never, matched as never)]).toEqual([0, 3]);
  expect(matched).toEqual([5]);
  const ignored: number[] = [];
  lib.ignored(false as never, ignored as never);
  lib.ignored(true as never, ignored as never);
  expect(ignored).toEqual([1, 2, 3, 2]);
});

// React's beginWork: `oldProps !== newProps || hasLegacyContextChanged() ||
// // Force a re-render if the implementation changed due to hot reload:
// (__DEV__ ? workInProgress.type !== current.type : false)`. Its port's
// comment, before an `||`'s operand of an `if`'s value, was lost (ADR 0444,
// amended). The `if`'s value is the `&&` it means, the user's decision (ADR
// 0879, amended).
test("a comment before an operand that's an if's value is before it", async () => {
  const { js, lib } = await compiled("comment-before-an-if-operand", `const DEV: bool = true;

pub fn changed(a: u8, b: u8, c: bool, t: u8, u: u8) -> bool {
    let mut did = false;
    if a != b
        || c
        // Force a re-render if the implementation changed due to hot reload:
        || (if DEV { t != u } else { false })
    {
        // If props or context changed, mark the fiber as having performed work.
        did = true;
    }
    did
}
`);
  expect(js).toContain(`    c ||
    // Force a re-render if the implementation changed due to hot reload:
    (DEV && t !== u)
  ) {
    // If props or context changed, mark the fiber as having performed work.
    did = true;`);
  expect([lib.changed(1 as never, 1 as never, false as never, 2 as never, 3 as never), lib.changed(1 as never, 1 as never, false as never, 2 as never, 2 as never)]).toEqual([true, false]);
});

// React's ReactDOMFormActions, `ReactDOMSharedInternals.d /*
// ReactDOMCurrentDispatcher */\n .r(/* requestFormReset */ form);`: a block
// comment ending a call chain's link's line is before the next link's `.`,
// and one right after a call's `(` is before its argument, each where it's
// written, as upstream has them (rd63).
test("block comments in a call chain are where they're written", async () => {
  const { js, lib } = await compiled("comments-in-a-chain", `use std::marker::PhantomData;

pub struct Internals(PhantomData<js::JsObject>);
pub struct Dispatcher(PhantomData<js::JsObject>);

impl Internals {
    #[cfg_attr(rust_js, rust_js::link_name = "get d")]
    pub fn d(&self) -> &'static Dispatcher {
        unreachable!()
    }
}

impl Dispatcher {
    #[cfg_attr(rust_js, rust_js::link_name = "r")]
    pub fn r(&self, form: f64) {
        let _ = form;
        unreachable!()
    }
}

pub fn reset(internals: &Internals, form: f64) {
    internals
        .d() /* CurrentDispatcher */
        .r(/* requestFormReset */ form);
}
`);
  expect(js).toContain("  internals.d /* CurrentDispatcher */\n    .r(/* requestFormReset */ form);\n");
  const reset: number[] = [];
  (lib.reset as (internals: unknown, form: number) => void)({ d: { r: (form: number) => reset.push(form) } }, 3);
  expect(reset).toEqual([3]);
});

// React's ReactDOMFlushSync, `ReactDOMSharedInternals.p /*
// ReactDOMCurrentUpdatePriority */ = DiscreteEventPriority;`, and
// ReactFizzConfigDOM's `ReactDOMSharedInternals.d /* ReactDOMCurrentDispatcher
// */ = { .. }`: a block comment ending the line before a setter's link, of
// what the setter sets, is after that place, before its `=`, as upstream has
// it, not after the `=`; not before an `==` a method's JS ends with.
test("a block comment before a setter's link is before its `=`", async () => {
  const { js, lib } = await compiled("comment-before-a-setter", `use std::marker::PhantomData;

pub struct Internals(PhantomData<js::JsObject>);

impl Internals {
    #[cfg_attr(rust_js, rust_js::link_name = "set p")]
    pub fn set_p(&self, p: f64) {
        let _ = p;
        unreachable!()
    }
    #[cfg_attr(rust_js, rust_js::link_name = "set d")]
    pub fn set_d(&self, d: &'static [f64]) {
        let _ = d;
        unreachable!()
    }
}

pub fn set(internals: &Internals, priority: f64) {
    internals /* CurrentUpdatePriority */
        .set_p(priority);
}

const DISCRETE: u8 = 2;

pub fn discrete(internals: &Internals) {
    internals /* CurrentUpdatePriority */
        .set_p(f64::from(DISCRETE));
}

pub fn same(name: &str, other: &str) -> bool {
    name /* the tag's */
        .eq(other)
}

pub fn dispatch(internals: &Internals) {
    internals /* CurrentDispatcher */
        .set_d(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0]);
}
`);
  expect(js).toContain("  internals.p /* CurrentUpdatePriority */ = priority;\n");
  // Of a value made by a call the JS has none of, `f64::from(DISCRETE)`'s
  // `DISCRETE`, as ReactDOMFlushSync's `i32::from(DISCRETE_EVENT_PRIORITY)`.
  expect(js).toContain("  internals.p /* CurrentUpdatePriority */ = DISCRETE;\n");
  expect(js).toContain("  internals.d /* CurrentDispatcher */ = [1, 2,");
  const internals: { p?: number; d?: number[] } = {};
  (lib.set as (internals: unknown, priority: number) => void)(internals, 2);
  (lib.dispatch as (internals: unknown) => void)(internals);
  expect([internals.p, internals.d?.length]).toEqual([2, 16]);
  // Not an `==` a method's JS is: there it's a comment as any other.
  expect(js).toContain("  return name /* the tag's */ === other;\n");
  expect([(lib.same as (a: string, b: string) => boolean)("a", "a"), (lib.same as (a: string, b: string) => boolean)("a", "b")]).toEqual([true, false]);
});

// And of an argument given by reference, `&str`, a reborrow MIR makes of
// it, which is where it's written too (rd205).
test("a block comment before an argument given by reference is before it", async () => {
  const { js, lib } = await compiled("comments-before-a-reference", `use std::marker::PhantomData;

pub struct Internals(PhantomData<js::JsObject>);
pub struct Dispatcher(PhantomData<js::JsObject>);

impl Internals {
    #[cfg_attr(rust_js, rust_js::link_name = "get d")]
    pub fn d(&self) -> &'static Dispatcher {
        unreachable!()
    }
}

impl Dispatcher {
    #[cfg_attr(rust_js, rust_js::link_name = "r")]
    pub fn r(&self, form: &str) {
        let _ = form;
        unreachable!()
    }
}

pub fn reset(internals: &Internals, form: &str) {
    internals
        .d() /* CurrentDispatcher */
        .r(/* requestFormReset */ form);
}
`);
  expect(js).toContain("  internals.d /* CurrentDispatcher */\n    .r(/* requestFormReset */ form);\n");
  const reset: string[] = [];
  (lib.reset as (internals: unknown, form: string) => void)({ d: { r: (form: string) => reset.push(form) } }, "f");
  expect(reset).toEqual(["f"]);
});


// A comment above a `_` arm that opens a block is before its `default`, as
// above the labels a `_` stands for, React's BeginWork `// The default is
// now forwards.` above `case 'forwards': default:`; above one that's a value
// it's its body's, as getIODescription's is (rd214, ADR 0590 amended).
test("a comment above a _ arm that opens a block is before its default", async () => {
  const { js, lib } = await compiled("default-comment-above", `pub fn tail_mode(mode: &str, set: &dyn Fn(u32)) {
    match mode {
        "backwards" => {
            set(1);
        }
        "together" => {
            set(2);
        }
        // The default is now forwards.
        _ => {
            set(3);
        }
    }
}
`);
  expect(js).toContain("      break;\n    // The default is now forwards.\n    default:\n      set(3);\n");
  const seen: number[] = [];
  const set = (n: number) => seen.push(n);
  const tailMode = lib.tailMode as (mode: string, set: (n: number) => void) => void;
  tailMode("together", set);
  tailMode("forwards", set);
  expect(seen).toEqual([2, 3]);
});

// React's ReactART sets its mode with a comment above the argument, an
// extern static, `Mode.setCurrent(\n  // Change to 'art/modes/dom' ..\n
// FastNoSideEffects,\n);`: there in the call, not above it, as one above a
// variable or a constant is (ADR 0708; rd421), though MIR reads the static
// through a pointer to it before the call.
test("a comment above a static given as an argument is above it in the call", async () => {
  const { js, lib } = await compiled("comment-above-a-static-argument", `use js::JsObject;

unsafe extern "Rust" {
    #[link_name = "Math"]
    safe static MATH: &'static JsObject;
}

#[cfg_attr(rust_js, rust_js::link_name = "Object.isFrozen")]
fn is_frozen(_object: &JsObject) -> bool {
    unreachable!()
}

fn is_math(object: &JsObject) -> bool {
    std::ptr::eq(object, MATH)
}

pub fn frozen() -> bool {
    is_frozen(
        // Change to 'art/modes/dom' for easier debugging via SVG
        MATH,
    )
}

pub fn math() -> bool {
    is_math(
        // The global one
        MATH,
    )
}
`);
  expect(js).toContain("  return Object.isFrozen(\n    // Change to 'art/modes/dom' for easier debugging via SVG\n    Math,\n  );");
  expect(js).toContain("  return isMath(\n    // The global one\n    Math,\n  );");
  expect(lib.frozen()).toBe(false);
  expect(lib.math()).toBe(true);
});

// React's ReactFlightPerformanceTrack, `if (description.charCodeAt(queryIdx
// - 1) === 47 /* "/" */) {`, whose port rustfmt writes with the comment on
// a line of its own before the `{`: after the condition's last code, as
// upstream has it (rd344); not one before a function's body.
test("a block comment between an `if`'s condition and its `{` is after the condition", async () => {
  const { js, lib } = await compiled("comment-before-a-block", `pub fn slash(description: &str, query_idx: usize) -> bool {
    if js::string::char_code_at(description, query_idx as f64 - 1.0) == 47.0
    /* "/" */
    {
        return true;
    }
    false
}

pub fn count(mut n: u32) -> u32 {
    let mut steps = 0;
    while n > 1
        && n != 7
    /* not seven */
    {
        n /= 2;
        steps += 1;
    }
    steps
}
`);
  expect(js).toContain(` === 47 /* "/" */) {\n`);
  expect(js).toContain(`n !== 7 /* not seven */) {\n`);
  const slash = lib.slash as (d: string, i: number) => boolean;
  expect([slash("a/b", 2), slash("ab", 2), (lib.count as (n: number) => number)(32)]).toEqual([true, false, 5]);
});

// React's ReactClientConsoleConfigBrowser, `const badgeStyle =\n  // We use
// a fixed background ..\n  'background: #e6e6e6;' +`: a comment before
// `concat!`'s first piece is after the `=`, before that piece, as upstream
// has it, as one before a later piece is after its `+` (rd342).
test("a comment before concat!'s first piece is before it", async () => {
  const { js, lib } = await compiled("comment-before-concat", `// Same badge styling as DevTools.
pub static BADGE_STYLE: &str = concat!(
    // We use a fixed background if light-dark is not supported, otherwise
    // we use a transparent background.
    "background: #e6e6e6;",
    "background: light-dark(rgba(0,0,0,0.1), rgba(255,255,255,0.25));",
    // Then its color.
    "color: #000000;",
    "border-radius: 2px"
);

pub fn style() -> String {
    let style = concat!(
        // Its padding first.
        "padding: 0 2px;",
        "margin: 0"
    );
    style.to_string()
}
`);
  expect(js).toContain(
    'export const BADGE_STYLE =\n  // We use a fixed background if light-dark is not supported, otherwise\n  // we use a transparent background.\n  "background: #e6e6e6;" +\n',
  );
  expect(js).toContain('  // Then its color.\n  "color: #000000;" +\n');
  expect(js).toContain('  const style =\n    // Its padding first.\n    "padding: 0 2px;" + "margin: 0";\n');
  expect([lib.BADGE_STYLE, (lib.style as () => string)()]).toEqual([
    "background: #e6e6e6;background: light-dark(rgba(0,0,0,0.1), rgba(255,255,255,0.25));color: #000000;border-radius: 2px",
    "padding: 0 2px;margin: 0",
  ]);
});

// A comment above an arm of a block that's no `switch`'s case, `// Nothing
// is released.` above `None => { DOWN.set(false); }`, whose first
// statement's span starts at `set`, past `DOWN.`: read past the arm's
// opening, before that statement, its `else`'s first (ADR 0905, amended,
// rd119).
test("a comment above an arm of a block of an if chain is before its first statement", async () => {
  const { js, lib } = await compiled("comment-above-a-chain-arm", `use std::cell::Cell;

thread_local! {
    static DOWN: Cell<bool> = const { Cell::new(false) };
}

pub fn pick(v: Option<u32>, out: &mut Vec<u32>) -> bool {
    match v {
        // A number is pressed.
        Some(n) => {
            DOWN.set(true);
            out.push(n);
        }
        // Nothing is released.
        None => {
            DOWN.set(false);
        }
    }
    DOWN.get()
}
`);
  expect(js).toContain("  // A number is pressed.\n  if (v != null) {\n");
  expect(js).toContain("  } else {\n    // Nothing is released.\n    down = false;\n");
  const out: number[] = [];
  const pick = lib.pick as (v: number | undefined, out: number[]) => boolean;
  expect([pick(4, out), pick(undefined, out), out]).toEqual([true, false, [4]]);
});
