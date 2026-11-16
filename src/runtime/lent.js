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
  // TODO(#2000): `BOX_PASSES_ON`'s, which the one catalog is to give both.
  boxed: ["next", "sizeHint", "nth", "last", "back", "nthBack", "len"],
  boxedUnsized: ["next", "sizeHint", "nth", "back", "nthBack", "len"],
};

// Every step a `&mut I` may pass on, and those std's defaults are of:
// `count` and `last`, which it doesn't.
const $stepNames = /* @__PURE__ */ (() => new Set(["count", "last"].concat($passedOn.lent, $passedOn.lentUnsized, $passedOn.boxed)))();

// And what says each is its own, `ownRfold` (`$ownStep`), and of which.
const $stepOf = /* @__PURE__ */ (() => new Map(Array.from($stepNames, (step) => [`own${step[0].toUpperCase()}${step.slice(1)}`, step])))();
const $ownStepNames = /* @__PURE__ */ (() => new Set($stepOf.keys()))();

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
  const passes = $passedOn[kind];
  const passed = (step) => (passes.includes(step) ? step : undefined);
  // A `Box<I>`'s, `boxed`, owns `it`: closing it closes `it` (#1995).
  const owns = kind === "boxed" || kind === "boxedUnsized";
  // Each step it passes on that `it` has, as `$forward` gives one, its own
  // where `it`'s is: its `advance_by` and `advance_back_by` where `it` has
  // its own (ADR 0913); its `fold` and `rfold`, where passed on, `it`'s
  // `try_fold` and `try_rfold`, never stopped. Given once, as a lend is
  // made: a compiled `filter`'s `next` lends its source each step, and a
  // getter of each step on each lend held runtime_long_writes past its
  // 10 s (2026-11-17).
  const fold = (init, f, unwind, ends) => it.tryFold(init, f, () => false, unwind, ends);
  const rfold = (init, f, unwind, ends) => it.tryRfold(init, f, () => false, unwind, ends);
  const lent = $forward(new $Steps(() => it.next(), owns ? () => it.return?.() : undefined), it, {
    nth: passed("nth"),
    advanceBy: passed("advanceBy") && Object.hasOwn(it, "advanceBy") ? "advanceBy" : undefined,
    tryFold: passed("tryFold"),
    fold: passed("fold") && it.tryFold !== undefined ? fold : undefined,
    last: passed("last"),
    nthBack: passed("nthBack"),
    advanceBackBy: passed("advanceBackBy") && Object.hasOwn(it, "advanceBackBy") ? "advanceBackBy" : undefined,
    tryRfold: passed("tryRfold"),
    rfold: passed("rfold") && it.tryRfold !== undefined ? rfold : undefined,
  });
  if (passed("back") && it.back !== undefined) lent.back = () => it.back();
  if (passed("len") && it.len !== undefined) lent.len = () => it.len();
  // Its `size_hint`, `it`'s, as std's `&mut I` passes it on (#745).
  lent.sizeHint = () => $sizeHint(it);
  // What it lends, whose facts are its own: `$slicing` gives it its
  // slice's steps, which it passes on (#745).
  lent.lends = it;
  // Of std's struct, compiled (ADR 0871): its fields `it`'s, read and
  // written through it, so its compiled bodies step `it` where it is, as
  // std's `&mut I` steps its `I`; what stops early closes `lent`, not it.
  if (it.$steps !== undefined) {
    // Its own first; then a step it passes on, or what says one is `it`'s
    // own, `it`'s as it is now, a back its `knowing` gives after it's lent
    // (ADR 0912), its `fold` and `rfold` by `it`'s `try_fold`s; never a step
    // it doesn't pass on, an unsized one's `rfold`, std's default (#1991);
    // and `it`'s fields, `iter` and `count$`, which a helper's name never
    // hides.
    const late = { fold: () => it.tryFold !== undefined && fold, rfold: () => it.tryRfold !== undefined && rfold };
    const get = (target, key) => {
      if (Object.hasOwn(target, key)) return target[key];
      if ($stepNames.has(key) || $ownStepNames.has(key)) {
        const step = $ownStepNames.has(key) ? $stepOf.get(key) : key;
        if (!passes.includes(step)) return target[key];
        if (key === step && late[key] !== undefined) return late[key]() || undefined;
        return it[key];
      }
      return (Object.hasOwn(it, key) ? it : target)[key];
    };
    return new Proxy(lent, {
      get,
      set: (target, key, value) => {
        (Object.hasOwn(target, key) || !Object.hasOwn(it, key) ? target : it)[key] = value;
        return true;
      },
    });
  }
  return lent;
}
