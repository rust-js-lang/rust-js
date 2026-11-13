// What a closure carries its drop under, JS's own `Symbol.dispose`, or the
// symbol registered for it where an engine has none yet (ADR 0484).
const $disposeKey = Symbol.dispose ?? Symbol.for("Symbol.dispose");

// A closure that leaves where it's made, its maker's `impl Fn`, carrying
// its drop, `drop`, which drops what it took (ADR 0484). One that carries a
// drop already, a `Box<dyn Fn>` generic code boxes again, `boxed<F:
// Fn>(f: F) -> Box<dyn Fn>` given one, is another allocation holding it: a
// function of its own that calls it carries the new drop, which drops the
// one it holds, its own drop kept (#1221).
function $dropping(f, drop) {
  const own = f[$disposeKey] === undefined ? f : (...args) => f(...args);
  own[$disposeKey] = drop;
  return own;
}

// A closure made elsewhere dropped: the drop it carries, `$dropping`'s, run.
// The compiler drops each closure where Rust does, once: a fold it was moved
// into drops it where std's does, before what's left, and its caller doesn't
// (#710, #734).
function $dispose(f) {
  const drop = f[$disposeKey];
  if (drop === undefined) return;
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
  return $dropping((...args) => $endsAfter(drop, () => f(...args)), drop);
}

// A JS iterator made a `dyn Iterator` of a value with a destructor: it,
// carrying its drop, `drop`, under `$disposeKey` as its own property, as a
// `dyn Fn`'s function does (ADR 0521, #1420). An own property: JS's
// `Iterator.prototype` has a `Symbol.dispose` of its own, which closes
// what inherits it, so one read through the prototype would close an
// iterator Rust doesn't drop, or one it closed already. One that carries a
// drop already, a `Box<dyn Iterator>` generic code boxes again, is another
// allocation holding it: one of its own that steps it carries the new drop,
// which drops the one it holds, its own kept.
function $droppingIterator(it, drop) {
  const lent = Object.hasOwn(it, $disposeKey) ? $lent(it) : it;
  // Stepped as Rust's, `$Steps`: its `map`, `filter`, `take`, `drop` and
  // `flatMap` std's adapters, which close what they step only where Rust
  // drops it, not JS's own helpers, which close it as their function
  // throws, dropping what the box holds before the fold's accumulator and
  // what a closure took (#1690).
  const own = lent instanceof $Steps ? lent : $stepsAround(lent);
  // Run once, wherever it's dropped first: its `Box`'s drop, or a consumer
  // that took it by value closing it as it ends, `count()`'s, `collect()`'s,
  // a `for`'s that breaks, as std's drops the `Box` it was given. A drop
  // that panics isn't run again.
  let dropped = false;
  const once = () => {
    if (dropped) return;
    dropped = true;
    drop();
  };
  Object.defineProperty(own, $disposeKey, { value: once, configurable: true, writable: true });
  const close = own.return;
  own.return = function (value) {
    const step = close === undefined ? { done: true, value } : close.call(this, value);
    once();
    return step;
  };
  return own;
}

// `it`, a JS iterator, stepped as a `$Steps`: its `next`, its `return`, and
// its hint, where it knows one (#1690).
function $stepsAround(it) {
  const steps = new $Steps(
    () => it.next(),
    () => it.return?.(),
  );
  steps.from = [it];
  steps.sizeHint = () => $sizeHint(it);
  return steps;
}

// A `dyn Iterator` dropped: the drop it carries, `$droppingIterator`'s, if
// any, its own, never its prototype's.
function $disposeIterator(it) {
  if (!Object.hasOwn(it, $disposeKey)) return;
  it[$disposeKey]();
}
