// A `Vec`'s capacity (ADR 0419), which JS's array hasn't: kept where a
// crate asks it, as std promises it, at least its length and what it was
// given, and never less where it's shortened. Its own number, not native
// Rust's growth's.
const $capacities = new WeakMap();

function $capacity(v) {
  return Math.max(v.length, $capacities.get(v) ?? 0);
}

// `Vec::with_capacity(n)`: none yet, room for `n`.
function $withCapacity(n) {
  const v = [];
  $capacities.set(v, n);
  return v;
}

// `v.reserve(n)`, `reserve_exact(n)`: room for `n` more.
function $reserve(v, additional) {
  $capacities.set(v, Math.max($capacity(v), v.length + additional));
}

// `v.shrink_to(n)`, `shrink_to_fit()` of `0`: no more than `n`, but its
// length.
function $shrinkTo(v, min) {
  $capacities.set(v, Math.max(v.length, Math.min($capacity(v), min)));
}

// Before it's shortened, `clear()`, `pop()` and the rest: what it has, kept.
function $keepCapacity(v) {
  $capacities.set(v, $capacity(v));
}

// Room for `count` items of `size` bytes each, which std checks wherever a
// capacity is asked (ADR 0774): past `isize::MAX` bytes, or past
// `usize::MAX` items, of no size too, a `capacity overflow` panic.
function $roomFor(count, size) {
  if (count > 2 ** 32 - 1 || count * size > 2 ** 31 - 1) throw new Error("capacity overflow");
}

// `it`, of what `Vec::from_iter` asks room for first, of `size` bytes each,
// as std's `SpecFromIter` does: a `TrustedLen`'s, `trusted`, its upper
// bound where its hint has one; of one whose type has no end, `endless`,
// a `capacity overflow` panic; else its lower bound and one more. A JS
// iterator's hint of no upper bound is no hint, not an endless one (ADR
// 0940). Of one that isn't `TrustedLen` but is `endless` once it gives an
// item, a `cycle()`'s, std steps it once first: with an item, its lower
// bound then is `usize::MAX`, so the same panic, as it unwinds the item
// dropped, `drop`, then what's left of it, closed (ADR 0940, amended).
function $roomForHint(it, size, trusted, endless = false, drop) {
  if (endless && trusted) throw new Error("capacity overflow");
  if (endless) {
    const first = it.next();
    if (first.done) return it;
    drop?.(first.value);
    it.return?.();
    throw new Error("capacity overflow");
  }
  const [lower, upper] = $sizeHint(it);
  if (trusted && upper !== undefined) $roomFor(upper, size);
  else $roomFor(Math.min(lower + 1, 2 ** 32 - 1), size);
  return it;
}

// `s.reserve(additional)` of a `String`: room for its bytes and as many
// more, its bytes counted only where they may not fit, as each of its
// UTF-16 units is at most three.
function $textRoomFor(s, additional) {
  if (additional > 2 ** 31 - 1 - 3 * s.length) $roomFor($byteLen(s) + additional, 1);
}

// Room for `count` entries of `size` bytes each, aligned to `align`, of a
// `HashMap` or `HashSet`, which std asks of hashbrown's table where a
// capacity is asked (ADR 0774, amended): its buckets, at least 1/8 empty,
// and a control byte each, and a group's more, past `isize::MAX` bytes, or
// more than `usize::MAX` of them, a `Hash table capacity overflow` panic.
function $tableRoomFor(count, size, align) {
  if (count === 0) return;
  const overflow = () => new Error("Hash table capacity overflow");
  // A group's control bytes, wasm32's.
  const width = 8;
  let buckets;
  // Of fewer than 15, 4, 8 or 16, hashbrown's; of entries of a byte or
  // none, it takes 8 at least, which fit as surely as 4 do.
  if (count < 15) {
    buckets = count < 4 ? 4 : count < 8 ? 8 : 16;
  } else {
    if (count * 8 > 2 ** 32 - 1) throw overflow();
    const adjusted = Math.floor((count * 8) / 7);
    buckets = 1;
    while (buckets < adjusted) buckets *= 2;
  }
  const ctrlAlign = Math.max(align, width);
  const ctrlOffset = Math.ceil((size * buckets) / ctrlAlign) * ctrlAlign;
  if (ctrlOffset + buckets + width > 2 ** 31 - 1 - (ctrlAlign - 1)) throw overflow();
}
