# react-native: React Native for rust-js

The globals React Native's host installs in its JS runtime, for a
program's Rust that runs in React Native, each typed as React Native's C++
bindings define it and written as the JS you'd write
([ADR 1083](../docs/decisions/1083-react-natives-host-globals-are-the-react-native-crates.md)):

```rust
use react_native::native_runtime_scheduler;

pub fn now() -> f64 {
    if js::is_defined(native_runtime_scheduler) {
        native_runtime_scheduler.unstable_now()
    } else {
        js::date::now()
    }
}
```

```js
export function now() {
  return typeof nativeRuntimeScheduler !== "undefined" ? nativeRuntimeScheduler.unstable_now() : Date.now();
}
```

What it has is what ports have needed: `nativeRuntimeScheduler`, the
scheduler React's `SchedulerNative` reads, its priorities, its
`unstable_scheduleCallback` and `unstable_cancelCallback` of the host's
own tasks, and its `unstable_shouldYield`, `unstable_requestPaint`,
`unstable_getCurrentPriorityLevel` and `unstable_now`.
