// What std's `&mut I` passes on to its `I`, by `$forward`'s names, each
// a step that is `I`'s own; every other is std's default over those,
// `$Steps`' and `back_steps.js`' (#1986, #1991). Of core's `impl Iterator
// for &mut I`, `impl DoubleEndedIterator for &mut I` and `impl
// ExactSizeIterator for &mut I`: of an `I: Sized`, `lent`, its `fold` and
// `rfold` by `I`'s `try_fold` and `try_rfold`, as their `RefSpec`
// specializations fold; of an unsized one, a `dyn`'s, `lentUnsized`,
// neither, std's defaults over its `next` and `next_back`; and of a
// `Box<I>`, `boxed` and `boxedUnsized` (#1601, #1995). The compiler's one
// catalog's, std's steps (crates/rust-js-lower/src/recognition/std_steps.rs, #2000,
// #2112), by `$forward`'s names: test/std-steps.test.ts makes the catalog
// from std itself, and its unit tests hold each list here to it.
const $passedOn = {
  lent: ["next", "sizeHint", "advanceBy", "nth", "fold", "tryFold", "back", "advanceBackBy", "nthBack", "rfold", "tryRfold", "len"],
  lentUnsized: ["next", "sizeHint", "advanceBy", "nth", "back", "advanceBackBy", "nthBack", "len"],
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
// What `it`, a lend of an iterator, `$lent`'s, lends, a lend's of a lend
// to its end; `it` itself of one that's no lend: the iterator a call of its
// own method through a `&mut` reaches, `it.by_ref().position(..)` of an
// `I` whose impl writes `position`, as Rust's `I::position` runs on the `I`
// (#2421).
function $referent(it) {
  while (it.lends !== undefined) it = it.lends;
  return it;
}

function $lent(it, kind = "lent") {
  // Of an iterable that's no iterator, an array a caller gives, a JS
  // iterator of it. Never `Iterator.from` of one: it calls a `$Steps`'
  // `[Symbol.iterator]()`, which arms what's lent to close at its end, as
  // a consumer that takes it by value does, so its lender's would be
  // dropped at its first `None` (#1508).
  if (typeof it.next !== "function") it = Iterator.from(it);
  // A sized `&mut` of what a `&mut` lends already, lent as the innermost
  // lends, as `Wrapper::lending` says, and an unsized one of an unsized
  // one: the lend itself, which a second one over std's compiled struct
  // would read its fields through twice, a dictionary's `lent` of a value
  // its caller lent (#2187).
  if ((kind === "lent" && (it.lentAs === "lent" || it.lentAs === "lentUnsized")) || (kind === "lentUnsized" && it.lentAs === kind)) return it;
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
  const lent = $forward(new $Steps(() => it.next(), owns ? () => $closeBare(it) : undefined), it, {
    nth: passed("nth"),
    advanceBy: passed("advanceBy") && $ownMethod(it, "advanceBy") ? "advanceBy" : undefined,
    tryFold: passed("tryFold"),
    fold: passed("fold") && it.tryFold !== undefined ? fold : undefined,
    last: passed("last"),
    nthBack: passed("nthBack"),
    advanceBackBy: passed("advanceBackBy") && $ownMethod(it, "advanceBackBy") ? "advanceBackBy" : undefined,
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
  lent.lentAs = kind;
  // A `Box`'s, which owns `it`, held: `it` held, as the box's drop is
  // what it holds', so a fold of the box drops `it` after its loop, not at
  // its end inside a step (#2232). A `&mut`'s hold stops at it. Not its
  // `from`, which says what std's `SourceIter` reaches, as no `Box` is one.
  if (owns) {
    lent.hold = () => {
      if (lent.held) return;
      lent.held = true;
      it.hold?.();
    };
  }
  // A `Box`'s, which owns `it`, dropped: `it` dropped, where it carries its
  // drop, a `Box<dyn Iterator>`'s (`$droppingIterator`), as the box's drop
  // runs what it holds' (#2186).
  if (owns && Object.hasOwn(it, $disposeKey)) lent[$disposeKey] = () => $disposeIterator(it);
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
