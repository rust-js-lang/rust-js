// A JS iterator lent as a `&mut` (ADR 0071): it steps `it`, and what takes
// from it and stops early, as `take` or a loop that breaks does, closes
// this, not `it`, which its lender goes on stepping. Its `nth` is `it`'s,
// as std's `&mut I` gives it, the crate's own where `$iterator` was given
// it; its `fold`, std's, of `next` (#162); its `tryFold`, `it`'s, as
// std's `&mut I` gives it, where `it` is std's adapter of one.
function $lent(it) {
  const lent = new $Steps(() => it.next());
  if (it.nth !== undefined) lent.nth = (k) => it.nth(k);
  if (it.tryFold !== undefined) lent.tryFold = (init, f, broke) => it.tryFold(init, f, broke);
  return lent;
}
