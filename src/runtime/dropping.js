// What a closure carries its drop under, JS's own `Symbol.dispose`, or the
// symbol registered for it where an engine has none yet (ADR 0484).
const $disposeKey = Symbol.dispose ?? Symbol.for("Symbol.dispose");

// A closure that leaves where it's made, its maker's `impl Fn`, carrying
// its drop, `drop`, which drops what it took (ADR 0484).
function $dropping(f, drop) {
  f[$disposeKey] = drop;
  return f;
}

// A closure made elsewhere dropped: the drop it carries, `$dropping`'s, run.
function $dispose(f) {
  f[$disposeKey]?.();
}

// A `Fn` or an `FnMut` closure made a `dyn FnOnce`: its call consumes it,
// so what it took drops as the call returns or unwinds, as std's
// `call_once` shim drops it; dropped uncalled, its drop too (#409).
function $droppingOnce(f, drop) {
  return $dropping((...args) => {
    try {
      return f(...args);
    } finally {
      drop();
    }
  }, drop);
}
