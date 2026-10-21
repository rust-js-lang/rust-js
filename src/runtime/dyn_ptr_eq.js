// `p == q` and `ptr::addr_eq(p, q)` of two pointers to a `dyn`, each its
// pair `{ value, impl }` (ADR 0562): the same value. Two that `==` can
// compare are of one trait, so one value's vtable is one.
function $dynPtrEq(p, q) {
  if (p === q) return true;
  if (p === null || q === null) return false;
  return p.value === q.value;
}
