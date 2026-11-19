// The steps an iterator rust-js makes has beside `next`, each std's
// `Iterator` or `DoubleEndedIterator` method an adapter may specialize, as
// a JS iterator's: `fold(init, f, unwind)`, `tryFold(init, f, broke,
// unwind)`, `rfold(init, f, unwind)`, `tryRfold(init, f, broke, unwind)`,
// `nth(k, drop)`, `nthBack(k)`, `last()`, `count(drop)`,
// `advanceBy(m, drop)` and `advanceBackBy(m, drop)`. A step it lacks is
// std's default, `$Steps`' (ADR 0669), or, from its back, `back_steps.js`'
// helpers'.
//
// `adapter` given each of the steps `steps` names, the one place one is
// given (ADR 1089): of a string, `source`'s step of that name, passed on,
// where `source` has it, with every argument it's given, so an item's drop
// and an accumulator's `unwind` always go on with it, and its own, the
// crate's or one that drops what it passes itself (`ownNth`, `ownFold`,
// ..), where `source`'s is; of a function, the adapter's own, std's
// specialization of it; of `undefined`, none, so it's std's default.
// Given `each`, what an adapter does to each item, a `map`'s, a fold passed
// on is given `each(f, unwind)` for its `f`, the `unwind` it's given; its
// own it isn't, as the adapter holds more than `source`, its function,
// which the crate's own `fold` that takes `source` doesn't drop.
function $forward(adapter, source, steps, each) {
  for (const name of Object.keys(steps)) {
    const by = steps[name];
    if (typeof by === "function") {
      adapter[name] = by;
    } else if (by !== undefined && source[by] !== undefined) {
      // Where a fold's `unwind` is, after its `f` and, of a `try_fold`,
      // its `broke`.
      const unwindAt = { fold: 0, rfold: 0, tryFold: 1, tryRfold: 1 }[by];
      adapter[name] =
        each === undefined || unwindAt === undefined
          ? (...args) => source[by](...args)
          : (init, f, ...rest) => source[by](init, each(f, rest[unwindAt]), ...rest);
      if (each === undefined && source[$ownStep(by)]) $owns(adapter, name);
    }
  }
  return adapter;
}

// `adapter` given std's own `DoubleEndedIterator` steps, `steps`, wherever
// it steps from its back, whatever `source` writes of them: std writes an
// adapter's `nth_back`, `rfold`, `try_rfold` and `advance_back_by` whatever
// its source writes, each asking its source's own step or the trait's
// default of it (`$nthBackOf`, `$foldBack`, `$tryRfoldOf`,
// `$advanceBackBy`). Given only where its source had that step, a `take`'s,
// an `enumerate`'s or a `skip`'s fell back to `$Steps`' default over its own
// `next_back`, which asks its source's length at each step where std's
// asks it once (#2136, #2140, #2141).
function $forwardBack(adapter, source, steps) {
  if (source.back === undefined) return adapter;
  return $forward(adapter, source, steps);
}

// The flag that says an iterator's `step` is its own, `ownNth` of `nth`:
// calling it runs one that drops what it passes, or takes the value and
// drops it, itself (ADR 1092 §4). Compiled code reads it by this name.
function $ownStep(step) {
  return `own${step[0].toUpperCase()}${step.slice(1)}`;
}

// `it`'s `steps` its own, where it's made: the crate's, a `Vec`'s slice
// steps, a compiled body's, or an adapter's that is so of itself, a
// flatten's `count`. The one place an own fact is written (#870).
function $owns(it, ...steps) {
  for (const step of steps) it[$ownStep(step)] = true;
  return it;
}

// `adapter`'s `steps`, each its own that passes it on to `source`'s of the
// same name: its own where `source`'s is, said where it's made, as a step
// `$forward` passes on by name is (#870).
function $ownsAs(adapter, source, ...steps) {
  return $owns(adapter, ...steps.filter((step) => source[$ownStep(step)]));
}
