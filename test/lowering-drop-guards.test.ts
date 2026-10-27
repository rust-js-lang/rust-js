// A guard whose `Drop` restores what its scope changed, as React restores
// it in a `finally` (ADRs 0589, 0693, 0705). Each test compiles a program
// and runs it.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

/** React's `runWithFiberInDEV`, its guard's `Drop` of `drop`'s body. */
const runWithFiber = (guard: string, drop: string) => `use std::cell::RefCell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(fiber))]
pub fn set_current_fiber(fiber: Option<String>) {
    CURRENT.set(fiber);
}

pub fn current_fiber() -> Option<String> {
    CURRENT.with_borrow(|current| current.clone())
}

${guard}

impl Drop for Restore {
    fn drop(&mut self) {
        ${drop}
    }
}

pub fn run_with_fiber_in_dev<T>(fiber: Option<String>, wrap: bool, callback: impl FnOnce() -> T) -> T {
    let previous_fiber = CURRENT.with_borrow(|current| current.clone());
    set_current_fiber(fiber);
    let _restore = Restore(previous_fiber);
    if wrap {
        return callback();
    }
    callback()
}
`;

/** Each way out of `runWithFiberInDEV` restores the fiber before it, once. */
async function restoresEach(dir: string) {
  const lib = await import(join(dir, "lib.js"));
  lib.setCurrentFiber("root");
  const seen: unknown[] = [];
  expect(lib.runWithFiberInDev("a", true, () => (seen.push(lib.currentFiber()), 1))).toBe(1);
  expect(lib.currentFiber()).toBe("root");
  expect(lib.runWithFiberInDev("b", false, () => (seen.push(lib.currentFiber()), 2))).toBe(2);
  expect(lib.currentFiber()).toBe("root");
  expect(() =>
    lib.runWithFiberInDev("c", true, () => {
      seen.push(lib.currentFiber());
      throw new RangeError("thrown");
    }),
  ).toThrow("thrown");
  expect([seen, lib.currentFiber()]).toEqual([["a", "b", "c"], "root"]);
}

// React's `runWithFiberInDEV` returns from inside its `try`, whose
// `finally` restores the fiber: `try { if (..) { return ..; } return
// callback(..); } finally { setCurrentFiber(previousFiber); }`. Its port's
// `return callback();` from the guard's scope gives the function's return
// place, which neither a closure nor the drop sets: returned in the `try`,
// whose `finally` drops the guard after the value's made, as Rust drops it
// (ADR 0693, amended), not a `try` of each way out and a `let result`.
test("an early return in a guard's scope returns in its try", async () => {
  const { js, dir } = compiled(
    "guard-early-return",
    runWithFiber("struct Restore {\n    previous: Option<String>,\n}", "set_current_fiber(self.previous.clone());").replace(
      "let _restore = Restore(previous_fiber);",
      "let _restore = Restore { previous: previous_fiber };",
    ),
  );
  expect(js).toContain(`  try {
    if (wrap) {
      return callback();
    }
    return callback();
  } finally {
    setCurrentFiber(previousFiber);
  }
`);
  expect(js).not.toContain("let result");
  expect(js.match(/finally/g)?.length).toBe(1);
  await restoresEach(dir);
});

// Its port's guard is a tuple struct, `struct Restore(Option<Fiber>)`: its
// one field, made of a variable nothing sets once the guard's made, reads
// as that variable, `setCurrentFiber(previousFiber)`, as a named field's
// does (ADR 0705), not a `const restore = previousFiber;` of its own. One
// set later is what it was when the guard was made, a `const` of its own.
test("a one-field guard of a variable reads as that variable", async () => {
  const { js, dir } = compiled(
    "guard-one-field-variable",
    runWithFiber("struct Restore(Option<String>);", "set_current_fiber(self.0.clone());"),
  );
  expect(js).toContain(`  try {
    if (wrap) {
      return callback();
    }
    return callback();
  } finally {
    setCurrentFiber(previousFiber);
  }
`);
  expect(js).not.toContain("restore");
  await restoresEach(dir);
  const later = compiled(
    "guard-one-field-set-later",
    `struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restore {}", self.0);
    }
}

pub fn run(n: u32) -> u32 {
    let mut previous = n;
    let _restore = Restore(previous);
    previous += 1;
    previous
}
`,
  );
  expect(later.js).toContain("const restore = previous;");
  expect(later.js).toContain("console.log(`restore ${restore}`);");
});

// Its port's `Drop` takes what the guard holds, `set_current_fiber(
// self.0.take())`, which writes the field `None` as it reads it: the
// guard's dropped once, in the `finally`, and nothing reads it after, so
// the write goes, and the guard is its field's variable, as upstream's
// `finally { setCurrentFiber(previousFiber); }` (ADR 0705, amended). A
// `Drop` that reads the field again after taking it keeps the write, and
// so does a scope that writes it before its drop reads it; a guard made in
// a loop is made fresh each time round.
test("a guard's drop that takes its field reads the variable it was made of", async () => {
  const { js, dir } = compiled(
    "guard-take",
    runWithFiber("struct Restore(Option<String>);", "set_current_fiber(self.0.take());"),
  );
  expect(js).toContain(`export function runWithFiberInDev(fiber, wrap, callback) {
  const previousFiber = current;
  setCurrentFiber(fiber);
  try {
    if (wrap) {
      return callback();
    }
    return callback();
  } finally {
    setCurrentFiber(previousFiber);
  }
}
`);
  await restoresEach(dir);
  const named = compiled(
    "guard-take-named",
    runWithFiber("struct Restore {\n    previous: Option<String>,\n}", "set_current_fiber(self.previous.take());").replace(
      "let _restore = Restore(previous_fiber);",
      "let _restore = Restore { previous: previous_fiber };",
    ),
  );
  expect(named.js).toContain("  } finally {\n    setCurrentFiber(previousFiber);\n  }\n");
  await restoresEach(named.dir);
  const again = compiled(
    "guard-take-again",
    `struct Twice(Option<String>);

impl Drop for Twice {
    fn drop(&mut self) {
        let first = self.0.take();
        let again = self.0.take();
        println!("{first:?} {again:?}");
    }
}

pub fn twice(name: Option<String>) {
    let _t = Twice(name);
}

struct TwiceNamed {
    held: Option<String>,
}

impl Drop for TwiceNamed {
    fn drop(&mut self) {
        let first = self.held.take();
        let again = self.held.take();
        println!("{first:?} {again:?}");
    }
}

pub fn twice_named(v: Option<String>) {
    let _t = TwiceNamed { held: v };
}

struct More(Option<bool>);

impl Drop for More {
    fn drop(&mut self) {
        println!("more {:?}", self.0.take());
    }
}

pub fn flushed(work: fn()) {
    let mut guard = More(Some(true));
    work();
    guard.0 = None;
    work();
}

pub fn looped(names: Vec<String>) {
    for name in names {
        let _more = More(Some(name.is_empty()));
        println!("{name}");
    }
}
`,
  );
  expect(again.js).toContain("  _t[0] = undefined;\n  const again = _t[0];\n");
  expect(again.js).not.toContain("_more[0] = undefined;");
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const lib = await import(join(again.dir, "lib.js"));
    lib.twice("a");
    lib.twiceNamed("n");
    lib.flushed(() => {});
    lib.looped(["", "b"]);
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(['Some("a") None', 'Some("n") None', "more None", "", "more Some(true)", "b", "more Some(false)"]);
});

// A copy the drop makes of its guard's field, `let old = self.0.clone();`,
// is the variable the field reads as, at each place the guard's dropped;
// where the drop names another of its own so, a closure's parameter, the
// copy is its own (ADR 0705, amended).
test("a drop's copy of its guard's field reads as the field's variable", async () => {
  const { js, dir } = compiled(
    "guard-field-copies",
    `struct Restore(Option<String>);

impl Drop for Restore {
    fn drop(&mut self) {
        let old = self.0.clone();
        println!("{old:?}");
    }
}

pub fn branches(c: bool, n: Option<String>) {
    let g = Restore(n);
    if c {
        drop(g);
        println!("a");
    } else {
        println!("b");
    }
}

struct Named(Option<String>);

impl Drop for Named {
    fn drop(&mut self) {
        let old = self.0.take();
        let f = |old: Option<String>| old.map(|o| o.len());
        println!("{:?} {old:?}", f(Some("xy".to_string())));
    }
}

pub fn named(n: Option<String>) {
    let _g = Named(n);
}
`,
  );
  expect(js).not.toContain("g[0]");
  expect(js.slice(0, js.indexOf("function named")).match(/const old/g)).toBeNull();
  expect(js).toContain("  const old = n;\n  const f = (old) =>");
  const logged: unknown[] = [];
  const log = console.log;
  console.log = (...args: unknown[]) => logged.push(args.join(" "));
  try {
    const lib = await import(join(dir, "lib.js"));
    lib.branches(true, "x");
    lib.branches(false, "y");
    lib.named("abc");
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(['Some("x")', "a", "b", 'Some("y")', 'Some(2) Some("abc")']);
});

// React's ReactFiberCallUserSpace, ported: a closure coerced to a `fn` in a
// `const`, `callComponentInDEV`'s, whose guard restores what it changed.
// Its `Drop` is its body there, as in a function or a boxed closure (ADR
// 0608, amended): upstream's `try { .. } finally { setIsRendering(..) }`.
test("a guard in a closure that's a const fn is its drop's body there", async () => {
  const { js, dir } = compiled(
    "drop-in-const-closure",
    `use std::cell::Cell;

thread_local! {
    pub static IS_RENDERING: Cell<bool> = const { Cell::new(false) };
}

pub fn set_is_rendering(rendering: bool) {
    IS_RENDERING.set(rendering);
}

struct Restore(bool);

impl Drop for Restore {
    fn drop(&mut self) {
        set_is_rendering(self.0);
    }
}

pub const CALL: fn(&dyn Fn() -> bool) -> bool = |f| {
    let was_rendering = IS_RENDERING.get();
    set_is_rendering(true);
    let _restore = Restore(was_rendering);
    f()
};

pub type Call = fn(&dyn Fn() -> bool) -> bool;

struct Frame {
    #[cfg_attr(rust_js, rust_js::name = "react_stack_bottom_frame")]
    react_stack_bottom_frame: Call,
}

#[cfg_attr(rust_js, rust_js::name = "callComponent")]
const CALL_COMPONENT: Frame = Frame {
    react_stack_bottom_frame: |f| {
        let was_rendering = IS_RENDERING.get();
        set_is_rendering(true);
        let _restore = Restore(was_rendering);
        f()
    },
};

pub const CALL_COMPONENT_IN_DEV: Call = CALL_COMPONENT.react_stack_bottom_frame;

pub fn rendering() -> bool {
    IS_RENDERING.get()
}
`,
  );
  // A struct's field of one, as React's \`callComponent\` is.
  expect(js).toContain(`const callComponent = {
  react_stack_bottom_frame(f) {
    const wasRendering = isRendering;
    setIsRendering(true);
    try {
      return f();
    } finally {
      setIsRendering(wasRendering);
    }
  },
};
`);
  expect(js).toContain(`export const CALL = (f) => {
  const wasRendering = isRendering;
  setIsRendering(true);
  try {
    return f();
  } finally {
    setIsRendering(wasRendering);
  }
};
`);
  expect(js).not.toContain("_drop");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.CALL(() => lib.rendering()), lib.rendering()]).toEqual([true, false]);
  expect([lib.CALL_COMPONENT_IN_DEV(() => lib.rendering()), lib.rendering()]).toEqual([true, false]);
  expect(() =>
    lib.CALL(() => {
      throw new Error("thrown");
    }),
  ).toThrow("thrown");
  expect(lib.rendering()).toBe(false);
});

// React's DEV dispatchers, `if (__DEV__) { HooksDispatcherOnMountInDEV = {
// useMemo(create, deps) { const prevDispatcher = ReactSharedInternals.H;
// try { .. } finally { ReactSharedInternals.H = prevDispatcher; } } } }`,
// ported: a thread-local of `if __DEV__ { Some(Dispatcher { .. }) } else {
// None }`. A guard in a closure of either branch is its drop's body there,
// as in a `const`'s (ADR 0608, amended).
test("a guard in a closure of a conditional thread-local is its drop's body there", async () => {
  const { js, dir } = compiled(
    "drop-in-conditional-thread-local",
    `use std::cell::Cell;

unsafe extern "Rust" {
    safe static __DEV__: bool;
}

thread_local! {
    static H: Cell<u32> = const { Cell::new(0) };
}

pub fn set_h(h: u32) {
    H.set(h);
}

pub fn h() -> u32 {
    H.get()
}

struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        set_h(self.0);
    }
}

pub struct Dispatcher {
    pub use_memo: fn(&dyn Fn() -> u32) -> u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static ON_MOUNT_IN_DEV: Option<Dispatcher> = if __DEV__ {
        Some(Dispatcher {
            use_memo: |create| {
                let prev_dispatcher = h();
                set_h(7);
                let _restore = Restore(prev_dispatcher);
                create()
            },
        })
    } else {
        None
    };
}

pub fn memo(create: &dyn Fn() -> u32) -> Option<u32> {
    ON_MOUNT_IN_DEV.with(|d| d.as_ref().map(|d| (d.use_memo)(create)))
}
`,
  );
  expect(js).toContain(`      useMemo(create) {
        const prevDispatcher = h();
        setH(7);
        try {
          return create();
        } finally {
          setH(prevDispatcher);
        }
      },`);
  expect(js).not.toContain("_drop");
  (globalThis as any).__DEV__ = true;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.memo(() => lib.h()), lib.h()]).toEqual([7, 0]);
  expect(() =>
    lib.memo(() => {
      throw new Error("thrown");
    }),
  ).toThrow("thrown");
  expect(lib.h()).toBe(0);
});

// React's DEV dispatchers as upstream writes them, made where the module
// loads, `if (__DEV__) { HooksDispatcherOnMountInDEV = { useMemo(..) {
// const prevDispatcher = ReactSharedInternals.H; try { .. } finally {
// ReactSharedInternals.H = prevDispatcher; } } }; }`, ported in
// `js::on_load!`: a guard in a closure of what the module runs as it loads
// is its drop's body there too (ADR 0608, amended).
test("a guard in a closure of what a module runs as it loads is its drop's body there", async () => {
  const { js, dir } = compiled(
    "drop-in-load-closure",
    `use std::cell::{Cell, RefCell};
use std::rc::Rc;

unsafe extern "Rust" {
    safe static __DEV__: bool;
}

thread_local! {
    static H: Cell<u32> = const { Cell::new(0) };
    static ON_MOUNT_IN_DEV: RefCell<Option<Rc<Dispatcher>>> = RefCell::new(js::null());
}

pub fn set_h(h: u32) {
    H.set(h);
}

pub fn h() -> u32 {
    H.get()
}

struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        set_h(self.0);
    }
}

pub struct Dispatcher {
    pub use_memo: fn(&dyn Fn() -> u32) -> u32,
}

js::on_load! {
    if __DEV__ {
        ON_MOUNT_IN_DEV.set(Some(Rc::new(Dispatcher {
            use_memo: |create| {
                let prev_dispatcher = h();
                set_h(7);
                let _restore = Restore(prev_dispatcher);
                create()
            },
        })));
    }
}

pub fn memo(create: &dyn Fn() -> u32) -> Option<u32> {
    let use_memo = ON_MOUNT_IN_DEV.with_borrow(|d| d.as_ref().map(|d| d.use_memo));
    use_memo.map(|use_memo| use_memo(create))
}
`,
  );
  expect(js).toContain(`    useMemo(create) {
      const prevDispatcher = h();
      setH(7);
      try {
        return create();
      } finally {
        setH(prevDispatcher);
      }
    },`);
  expect(js).not.toContain("_drop");
  (globalThis as any).__DEV__ = true;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.memo(() => lib.h()), lib.h()]).toEqual([7, 0]);
  expect(() =>
    lib.memo(() => {
      throw new Error("thrown");
    }),
  ).toThrow("thrown");
  expect(lib.h()).toBe(0);
});

// React's applyDerivedStateFromProps: `setIsStrictModeForDevtools(true);
// try { // Invoke the function an extra time to help detect side-effects.
// partialState = getDerivedStateFromProps(..); } finally { .. }`. The `try`
// starts where the guard's `let` is: a comment after the `let` is inside
// it, one before it, or a blank line, before it (ADR 0865, amended).
test("a guard's try starts at its let, a comment after the let inside it", async () => {
  const { js, dir } = compiled(
    "guard-try-at-let",
    `use std::cell::Cell;

pub struct Exit<'a>(&'a Cell<bool>);

impl Drop for Exit<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

pub fn derive(strict: &Cell<bool>, f: &dyn Fn() -> u32) -> u32 {
    let mut partial_state = f();
    strict.set(true);
    let _exit = Exit(strict);
    // Invoke the function an extra time to help detect side-effects.
    partial_state += f();
    partial_state
}

pub fn before(strict: &Cell<bool>, f: &dyn Fn() -> u32) -> u32 {
    strict.set(true);
    // Set apart, as the guard is.
    let _exit = Exit(strict);
    f()
}
`,
  );
  expect(js).toContain(
    "  strict.value = true;\n  try {\n    // Invoke the function an extra time to help detect side-effects.\n    partialState = (partialState + f()) >>> 0;\n",
  );
  expect(js).toContain("  strict.value = true;\n  // Set apart, as the guard is.\n  try {\n    return f();\n");
  const lib = await import(join(dir, "lib.js"));
  const strict = { value: false };
  const seen: boolean[] = [];
  const f = () => (seen.push(strict.value), 2);
  expect([lib.derive(strict, f), strict.value, lib.before(strict, f), strict.value, seen]).toEqual([
    4,
    false,
    2,
    false,
    [false, true, true],
  ]);
});
