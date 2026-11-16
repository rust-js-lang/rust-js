// What std's `&mut I` passes on to its `I`, by `$forward`'s names, each
// a step that is `I`'s own; every other is std's default over those,
// `$Steps`' and `back_steps.js`' (#1986, #1991). Of core's `impl Iterator
// for &mut I`, `impl DoubleEndedIterator for &mut I` and `impl
// ExactSizeIterator for &mut I`: of an `I: Sized`, `lent`, its `fold` and
// `rfold` by `I`'s `try_fold` and `try_rfold`, as their `RefSpec`
// specializations fold; of an unsized one, a `dyn`'s, `lentUnsized`,
// neither, std's defaults over its `next` and `next_back`. A `Box<I>`'s
// are `BOX_PASSES_ON`'s (src/lower/recognition/iterators.rs, #1601). Facts
// of std's source: test/passed-on.test.ts reads each impl's items in
// rust-src and fails where a list here, or `BOX_PASSES_ON`, differs, as a
// toolchain's std may change them.
const $passedOn = {
  lent: ["next", "sizeHint", "advanceBy", "nth", "fold", "tryFold", "back", "advanceBackBy", "nthBack", "rfold", "tryRfold", "len"],
  lentUnsized: ["next", "sizeHint", "advanceBy", "nth", "back", "advanceBackBy", "nthBack", "len"],
};

// Every step a `&mut I` may pass on, and those std's defaults are of:
// `count` and `last`, which it doesn't.
const $stepNames = /* @__PURE__ */ (() => new Set(["count", "last"].concat($passedOn.lent, $passedOn.lentUnsized)))();

// A JS iterator lent as a `&mut` (ADR 0071), of a sized iterator or not,
// `kind`, `$passedOn`'s: it steps `it`, and what takes from it and stops
// early, as `take` or a loop that breaks does, closes this, not `it`,
// which its lender goes on stepping. The one owner of what std's `&mut I`
// passes on (`core/src/iter/traits/iterator.rs:4260`, `double_ended.rs:446`,
// ADR 1092 §3), each `it`'s, the crate's own where `$iterator` was given
// it; its `fold` and `rfold`, where passed on, `it`'s `try_fold` and
// `try_rfold`, never stopped, as `&mut I`'s `spec_fold` is for a sized `I`
// (#162, #1986): an inclusive range's ends it exhausted at its last (#414).
// Its `count` and `last` std's defaults, of those, so none reaches `it`'s
// own, which would take it from its lender (#747, #749).
function $lent(it, kind = "lent") {
  // Of an iterable that's no iterator, an array a caller gives, a JS
  // iterator of it. Never `Iterator.from` of one: it calls a `$Steps`'
  // `[Symbol.iterator]()`, which arms what's lent to close at its end, as
  // a consumer that takes it by value does, so its lender's would be
  // dropped at its first `None` (#1508).
  if (typeof it.next !== "function") it = Iterator.from(it);
  const lent = new $Steps(() => it.next());
  // Each step it passes on, `it`'s as `it` is when it's asked, so one a
  // compiled struct is given after, its `knowing`'s back, is too; where
  // `it` has none, std's default, `$Steps`'. Written over, it's what's
  // written, as an adapter's own step is.
  const defaults = Object.getPrototypeOf(lent);
  const pass = (name, step) =>
    Object.defineProperty(lent, name, {
      get: () => step() ?? defaults[name],
      set: (value) => Object.defineProperty(lent, name, { value, writable: true, configurable: true, enumerable: true }),
      configurable: true,
      enumerable: true,
    });
  const called = (name) => () => (it[name] === undefined ? undefined : (...args) => it[name](...args));
  for (const name of $passedOn[kind]) {
    if (name === "next" || name === "sizeHint") continue;
    // Its `fold` and `rfold`, `it`'s `try_fold` and `try_rfold`, never
    // stopped.
    if (name === "fold") pass(name, () => (it.tryFold === undefined ? undefined : (init, f, unwind) => it.tryFold(init, f, () => false, unwind)));
    else if (name === "rfold") pass(name, () => (it.tryRfold === undefined ? undefined : (init, f, unwind, ends) => it.tryRfold(init, f, () => false, unwind, ends)));
    // Its `advance_by` and `advance_back_by`, `it`'s own where it has one
    // (ADR 0913); else std's default, which `it`'s would be too.
    else if (name === "advanceBy" || name === "advanceBackBy") pass(name, () => (Object.hasOwn(it, name) ? (...args) => it[name](...args) : undefined));
    else {
      pass(name, called(name));
      // And whether it's `it`'s own, the crate's, which drops what it
      // passes itself (ADR 1092 §4).
      pass($ownStep(name), () => it[$ownStep(name)]);
    }
  }
  // Its `size_hint`, `it`'s, as std's `&mut I` passes it on (#745).
  lent.sizeHint = () => $sizeHint(it);
  // What it lends, whose facts are its own: `$slicing` gives it its
  // slice's steps, which it passes on (#745).
  lent.lends = it;
  // Of std's struct, compiled (ADR 0871): its fields `it`'s, read and
  // written through it, so its compiled bodies step `it` where it is, as
  // std's `&mut I` steps its `I`; what stops early closes `lent`, not it.
  if (it.$steps !== undefined) {
    // Its own steps first, then `it`'s fields, `iter` and `count$`, which
    // a helper's name never hides (ADR 0912). Never a step of `it`'s it
    // doesn't pass on, nor what says one is its own, `ownRfold`: an
    // unsized one's `rfold`, std's default, isn't its struct's (#1991).
    const step = (key) => typeof key === "string" && ($stepNames.has(key) || /^own[A-Z]/.test(key));
    const of = (target, key) => (Object.hasOwn(target, key) || step(key) || !Object.hasOwn(it, key) ? target : it);
    return new Proxy(lent, {
      get: (target, key) => of(target, key)[key],
      set: (target, key, value) => {
        of(target, key)[key] = value;
        return true;
      },
    });
  }
  return lent;
}
