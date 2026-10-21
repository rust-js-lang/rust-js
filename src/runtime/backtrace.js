// `std::backtrace::Backtrace` (ADR 0568): disabled unless forced, as Rust's
// is unless `RUST_BACKTRACE` asks, which JS has no environment to:
// `undefined`; forced, `Backtrace::force_capture()`, JS's own stack, its
// frames after the error's first line.
function $backtrace() {
  return (new Error().stack ?? "").split("\n").slice(1).join("\n");
}

// `b.status()`: `Disabled` of `undefined`, else `Captured`.
function $backtraceStatus(backtrace) {
  return backtrace === undefined ? "Disabled" : "Captured";
}

// `{}` of a backtrace, `disabled backtrace` or its frames; `{:?}`,
// `<disabled>` or its frames, as std shows them.
function $backtraceShown(backtrace, debug) {
  if (backtrace === undefined) return debug ? "<disabled>" : "disabled backtrace";
  return backtrace;
}
