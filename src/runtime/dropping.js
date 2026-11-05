// What a closure carries its drop under, JS's own `Symbol.dispose`, or the
// symbol registered for it where an engine has none yet (ADR 0484).
const $disposeKey = Symbol.dispose ?? Symbol.for("Symbol.dispose");

// A closure that leaves where it's made, its maker's `impl Fn`, carrying
// its drop, `drop`, which drops what it took (ADR 0484).
function $dropping(f, drop) {
  f[$disposeKey] = drop;
  return f;
}

// A closure made elsewhere dropped: the drop it carries, `$dropping`'s, run,
// once, as Rust drops a value once: a fold it was moved into drops it where
// std's does, before what's left, and its caller's drop after does nothing
// (#710).
function $dispose(f) {
  const drop = f[$disposeKey];
  if (drop === undefined) return;
  f[$disposeKey] = undefined;
  drop();
}

// A borrowed function made a `dyn Fn`, `Rc::new(&f)`'s: one of its own that
// calls it, carrying no drop, as what it borrows isn't its to drop (ADR
// 0521, amended).
function $borrowed(f) {
  return (...args) => f(...args);
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
