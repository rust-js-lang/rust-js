// A guard in a closure is its drop's body there, as in a function or a
// boxed closure (ADR 0608, amended): a closure coerced to a `fn` in a
// `const`, one of either branch of a conditional thread-local, and one of
// what a module runs as it loads, each upstream's `try { .. } finally {
// .. }`. Each test compiles a program and runs it.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8")), dir };
}

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
    } catch (error) {
      throw $unwindsOn(error);
    } finally {
      if ($live()) {
        setIsRendering(wasRendering);
      }
    }
  },
};
`);
  expect(js).toContain(`export const CALL = (f) => {
  const wasRendering = isRendering;
  setIsRendering(true);
  try {
    return f();
  } catch (error) {
    throw $unwindsOn(error);
  } finally {
    if ($live()) {
      setIsRendering(wasRendering);
    }
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
        } catch (error) {
          throw $unwindsOn(error);
        } finally {
          if ($live()) {
            setH(prevDispatcher);
          }
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
      } catch (error) {
        throw $unwindsOn(error);
      } finally {
        if ($live()) {
          setH(prevDispatcher);
        }
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
