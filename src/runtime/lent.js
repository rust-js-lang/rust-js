// A JS iterator lent as a `&mut` (ADR 0071): it steps `it`, and what takes
// from it and stops early, as `take` or a loop that breaks does, closes
// this, not `it`, which its lender goes on stepping. Its `nth` is `it`'s,
// as std's `&mut I` gives it, the crate's own where `$iterator` was given
// it; its `fold`, std's, of `next` (#162); its `tryFold`, `it`'s, as
// std's `&mut I` gives it, where `it` is std's adapter of one.
function $lent(it) {
  const lent = new $Steps(() => it.next());
  if (it.nth !== undefined) lent.nth = (k, drop) => it.nth(k, drop);
  if (it.ownNth) lent.ownNth = true;
  if (it.tryFold !== undefined) lent.tryFold = (init, f, broke) => it.tryFold(init, f, broke);
  // Of std's struct, compiled (ADR 0871): its fields `it`'s, read and
  // written through it, so its compiled bodies step `it` where it is, as
  // std's `&mut I` steps its `I`; what stops early closes `lent`, not it.
  if (it.$steps !== undefined) {
    // Its own steps first, then `it`'s fields, `iter` and `count`, which a
    // helper's name, `$Steps`' `count`, never hides.
    const of = (target, key) => (Object.hasOwn(target, key) || !Object.hasOwn(it, key) ? target : it);
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
