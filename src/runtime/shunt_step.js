// One step of what a collect into a `Result` or an `Option` steps, as
// std's `GenericShunt::next` is: `try_for_each(ControlFlow::Break)`, its
// source's `try_fold` broken at its first item. So a source whose
// `try_fold` steps otherwise than its `next` is stepped as Rust steps it:
// a `skip(n)`'s, its source's `nth(n - 1)` then its `try_fold`, where its
// `next` is `nth(n)`; a `step_by`'s, its first item by `next` (#1004). Of
// one with no `tryFold`, a JS iterator's, its `next`, which that is.
function $shuntStep(it) {
  if (typeof it.tryFold !== "function") return it.next();
  let item;
  const got = it.tryFold(
    false,
    (_, x) => {
      item = x;
      return true;
    },
    (hit) => hit,
  );
  if (got) return { done: false, value: item };
  // Ended, it's done with, as a step of its `next` that's done closes it,
  // where what it holds drops as the collect's temporary does; held, its
  // owner drops it (#206).
  if (!it.held) it.return?.();
  return { done: true, value: undefined };
}
