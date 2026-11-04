// The react-native crate (ADR 1083, #562): React Native's host globals, a
// binding crate beside webapi and node. Its `nativeRuntimeScheduler`, which
// React's SchedulerNative reads in Scheduler's place: a global that may be
// absent, `js::is_defined` of it `typeof nativeRuntimeScheduler !==
// "undefined"`, its priorities read, its functions called and read as the
// values upstream exports. Run against a host's stand-in of it, and with
// none.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReactNative, compiler, fixture, root } from "./support";

let native = "";
beforeAll(() => {
  native = buildReactNative();
}, 600_000);

test("React Native's nativeRuntimeScheduler is a global that may be absent, read as upstream reads it", async () => {
  const dir = fixture("react-native-scheduler");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use react_native::{Callback, NativeScheduler, Task, native_runtime_scheduler};

thread_local! {
    static RUNS: Cell<u32> = const { Cell::new(0) };
    pub static UNSTABLE_NOW: Option<&'static dyn Fn() -> f64> = if js::is_defined(native_runtime_scheduler) {
        Some(js::method((native_runtime_scheduler, NativeScheduler::unstable_now)))
    } else {
        None
    };
}

pub fn priorities() -> [u8; 5] {
    [
        native_runtime_scheduler.unstable_immediate_priority(),
        native_runtime_scheduler.unstable_user_blocking_priority(),
        native_runtime_scheduler.unstable_normal_priority(),
        native_runtime_scheduler.unstable_low_priority(),
        native_runtime_scheduler.unstable_idle_priority(),
    ]
}

pub fn schedule() -> &'static Task {
    native_runtime_scheduler.unstable_schedule_callback(
        native_runtime_scheduler.unstable_normal_priority(),
        Callback(Box::new(|did_timeout| {
            RUNS.set(RUNS.get() + if did_timeout { 10 } else { 1 });
            None
        })),
    )
}

pub fn cancel(task: &Task) {
    native_runtime_scheduler.unstable_cancel_callback(task);
}

pub fn runs() -> u32 {
    RUNS.get()
}

pub fn state() -> (u8, bool) {
    native_runtime_scheduler.unstable_request_paint();
    (
        native_runtime_scheduler.unstable_get_current_priority_level(),
        native_runtime_scheduler.unstable_should_yield(),
    )
}

pub fn now() -> f64 {
    if js::is_defined(native_runtime_scheduler) {
        native_runtime_scheduler.unstable_now()
    } else {
        -1.0
    }
}
`);
  const built = runSync(
    [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--",
      "--extern", `js=${join(native, "libjs.rmeta")}`, "--extern", `react_native=${join(native, "libreact_native.rmeta")}`, "-L", native],
    root,
    600_000,
  );
  expect(built.code, built.stderr).toBe(0);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    'export const unstableNow =\n  typeof nativeRuntimeScheduler !== "undefined" ? nativeRuntimeScheduler.unstable_now : undefined;',
  );
  expect(js).toContain("    nativeRuntimeScheduler.unstable_ImmediatePriority,\n");
  expect(js).toContain(
    "  return nativeRuntimeScheduler.unstable_scheduleCallback(\n    nativeRuntimeScheduler.unstable_NormalPriority,\n    (didTimeout) => {\n",
  );
  expect(js).toContain("  nativeRuntimeScheduler.unstable_cancelCallback(task);");
  expect(js).toContain('  return typeof nativeRuntimeScheduler !== "undefined" ? nativeRuntimeScheduler.unstable_now() : -1;');

  // A host's stand-in, as React Native's C++ installs it.
  const tasks: { callback: (didTimeout: boolean) => unknown; canceled: boolean }[] = [];
  const calls: string[] = [];
  const host = {
    unstable_ImmediatePriority: 1,
    unstable_UserBlockingPriority: 2,
    unstable_NormalPriority: 3,
    unstable_LowPriority: 4,
    unstable_IdlePriority: 5,
    unstable_scheduleCallback(priority: number, callback: (didTimeout: boolean) => unknown) {
      calls.push(`schedule ${priority}`);
      const task = { callback, canceled: false };
      tasks.push(task);
      return task;
    },
    unstable_cancelCallback(task: { canceled: boolean }) {
      task.canceled = true;
    },
    unstable_getCurrentPriorityLevel: () => 2,
    unstable_shouldYield: () => true,
    unstable_requestPaint: () => calls.push("paint"),
    unstable_now: () => 42.5,
  };
  const global = globalThis as { nativeRuntimeScheduler?: typeof host };
  global.nativeRuntimeScheduler = host;
  try {
    const lib = await import(join(dir, "lib.js"));
    expect(lib.unstableNow).toBe(host.unstable_now);
    expect(lib.priorities()).toEqual([1, 2, 3, 4, 5]);
    const first = lib.schedule();
    const second = lib.schedule();
    lib.cancel(second);
    expect([first.canceled, second.canceled]).toEqual([false, true]);
    expect(first.callback(false)).toBeUndefined();
    first.callback(true);
    expect(lib.runs()).toBe(11);
    expect(lib.state()).toEqual([2, true]);
    expect(calls).toEqual(["schedule 3", "schedule 3", "paint"]);
    expect(lib.now()).toBe(42.5);
    // Where the host has none, as every host but React Native's.
    delete global.nativeRuntimeScheduler;
    expect(lib.now()).toBe(-1);
  } finally {
    delete global.nativeRuntimeScheduler;
  }
});
