// `iter::repeat(value)`: a clone of `value` each step, and `value` dropped,
// `drop`, when it's closed, JS's `return()`, once, as Rust drops it with its
// iterator (ADR 0422). A `$Steps`, whose `take` closes it though a clone
// throws, a panic, as Rust drops a `Take` that holds it as it unwinds,
// where JS's own `take` would be done with it (#708). Its hint std's,
// `(usize::MAX, None)` (#246). Its lifecycle `$closing`'s, `$close`'s,
// never taken (#870).
function $repeating(value, clone = (value) => value, drop) {
  const it = new $Steps(
    () => ({ done: false, value: clone(value) }),
    () => $close(it, () => drop?.(value)),
  );
  // Its value, std's `Repeat`'s one field, `element`, which std's compiled
  // code reads where it inlines `Repeat::next`, a held `Take<Repeat<T>>`'s
  // `next_back` (take.rs:338), cloned there as `next` clones it (#1139).
  it.element = value;
  // Its clone, std's derived `Clone`, of a clone of its value, as a closure
  // cloned apart carries its own, `$clone`'s (repeat.rs:71, #1139).
  $cloning(it, () => $repeating(clone(value), clone, drop));
  it.sizeHint = () => [2 ** 32 - 1, undefined];
  // Std's `Repeat::advance_by`, which makes no item, so clones none: what
  // a `take(n)`'s `count` asks, and an `nth`, a clone of the next alone,
  // `$Steps`' of it (repeat.rs:92, #1124). And its `count` and `last`,
  // which panic, as it never ends, where std's default would step it
  // forever, `value` dropped as the panic unwinds, as `count(self)` drops
  // it (repeat.rs:110, #1138).
  const infinite = () =>
    $unwinds(
      () => {
        throw new Error("iterator is infinite");
      },
      () => it.return(),
    );
  $forward(it, undefined, { advanceBy: () => 0, count: infinite, last: infinite });
  // Std's `Take<Repeat<T>>`, of `n` items each alike: its back steps its
  // front steps, its length what it has left (take.rs:335, #1139).
  const take = it.take;
  it.take = (n) => {
    const taken = take.call(it, n);
    return $backAsFront(taken, () => taken.sizeHint()[0]);
  };
  return $owns(it, "nth", "last", "count");
}

// `it`, whose every step from its back is its step from its front, as
// std's `RepeatN` and `Take<Repeat<T>>` write theirs, of items each alike:
// `next_back` its `next`, `nth_back` its `nth`, `advance_back_by` its
// `advance_by`; and its length, `len` (repeat_n.rs:175, take.rs:335,
// #1139). Its `rfold` and `try_rfold`, std's `fold` and `try_fold`, are
// what folding it by `next_back` gives, which is `next`.
function $backAsFront(it, len) {
  it.back = () => it.next();
  it.len = len;
  $forward(it, it, { nthBack: "nth", advanceBackBy: "advanceBy" });
  return it;
}

// `iter::repeat_n(value, count)`: a clone of `value` each step but the
// last, which is `value` itself, as std's moves it; of a count of none,
// `value` dropped at once, and else when it's closed before its last
// (ADR 0513). A `$Steps`, as `$repeating` is (#708). Its hint exact, what
// it has left, as std's (#246).
function $repeatingN(value, count, clone = (value) => value, drop) {
  let left = count;
  if (left === 0) drop?.(value);
  const it = new $Steps(
    () => {
      if (left === 0) return { done: true, value: undefined };
      left--;
      return { done: false, value: left === 0 ? value : clone(value) };
    },
    () => {
      if (left > 0) {
        left = 0;
        drop?.(value);
      }
    },
  );
  it.sizeHint = () => [left, left];
  // Std's `RepeatN`'s own steps, which clone none of what they pass
  // (repeat_n.rs:140, #1124): `advance_by` counts down, and past its end
  // drops `value`; `nth` a clone of the next alone, or `value` itself at
  // its last, `$Steps`' of it; `last`, `value` itself; `count`, what it has
  // left, `value` dropped with it.
  const advanceBy = (m) => {
    if (m < left) {
      left -= m;
      return 0;
    }
    const short = m - left;
    if (left > 0) {
      left = 0;
      drop?.(value);
    }
    return short;
  };
  const last = () => {
    if (left === 0) return { done: true, value: undefined };
    left = 0;
    return { done: false, value };
  };
  const countLeft = () => {
    const counted = left;
    if (left > 0) {
      left = 0;
      drop?.(value);
    }
    return counted;
  };
  $forward(it, undefined, { advanceBy, last, count: countLeft });
  $backAsFront(it, () => left);
  // Its clone, std's derived `Clone` of its count and its `Option` of its
  // value: a clone of its value where it has one left, none where it's
  // done, which drops nothing (repeat_n.rs:68, #1579).
  $cloning(it, () => (left > 0 ? $repeatingN(clone(value), left, clone, drop) : $repeatingN(undefined, 0)));
  return $owns(it, "nth", "last", "count");
}
