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
// as std's `SpecFromIter` does. Of a `TrustedLen`'s, `trusted`, its hint
// first: its upper bound where it has one; of one whose type has no end,
// `endless`, a `capacity overflow` panic. A JS iterator's hint of no
// upper bound is no hint, not an endless one (ADR 0940). Of another,
// std's `SpecFromIterNested`: its first item by `next`, then, where it
// has one, its hint's lower bound and one more; of none, nothing asked
// and nothing stepped again, an empty `Vec`. Of one `endless` once it
// gives an item, a `cycle()`'s, that lower bound is `usize::MAX`, so the
// same panic (ADR 0940, amended). What it gives is `it` with that item
// first (#724). Where a step throws, a panic, `it` is closed, as
// `from_iter` drops what it took as it unwinds; where asking its room
// does, the item taken is dropped first, `drop`, as it was taken last:
// so `$collectDropping` of what it gives closes `it` wherever it ends
// (#660). And as what it gives is stepped, where the `Vec` std makes is
// full, the hint is asked again, as the `Vec` grows (#728).
function $roomForHint(it, size, trusted, endless = false, drop) {
  const unwound = (asked, before) => {
    try {
      return asked();
    } catch (error) {
      try {
        before?.();
      } finally {
        it.return?.();
      }
      throw error;
    }
  };
  if (trusted) {
    unwound(() => {
      if (endless) throw new Error("capacity overflow");
      const [lower, upper] = $sizeHint(it);
      $roomFor(upper !== undefined ? upper : Math.min(lower + 1, 2 ** 32 - 1), size);
    });
    return it;
  }
  // Its capacity, std's: what it asks room for first, the hint's lower
  // bound and one more, or `MIN_NON_ZERO_CAP` of items of `size` bytes if
  // more (raw_vec/mod.rs `min_non_zero_cap`); of items of no size, any
  // number, so it's never full.
  const least = size === 1 ? 8 : size <= 1024 ? 4 : 1;
  const max = 2 ** 32 - 1;
  let capacity;
  const first = unwound(() => it.next());
  if (!first.done) {
    unwound(
      () => {
        if (endless) throw new Error("capacity overflow");
        const [lower] = $sizeHint(it);
        capacity = size === 0 ? Infinity : Math.max(least, Math.min(lower + 1, max));
        $roomFor(Math.min(lower + 1, max), size);
      },
      () => drop?.(first.value),
    );
  }
  // How many it has given, which the `Vec` holds.
  let length = 0;
  const steps = new $Steps(
    () => {
      if (first.done) return first;
      if (length === 0) {
        length = 1;
        return first;
      }
      // Std's `extend_desugared`: each item by `next`, and where the `Vec`
      // is full, its hint asked again, room for its lower bound and one
      // more reserved, `grow_amortized`'s, at least twice what it had
      // (vec/mod.rs `extend_desugared`, raw_vec/mod.rs `grow_amortized`).
      // Where that throws, a panic, the item just taken is dropped as the
      // loop's frame unwinds, before what's left and what's collected
      // (#728).
      const step = it.next();
      if (step.done) return step;
      if (length === capacity) {
        try {
          const [lower] = $sizeHint(it);
          const required = length + Math.min(lower + 1, max);
          if (required > max) throw new Error("capacity overflow");
          capacity = Math.max(capacity * 2, required, least);
          $roomFor(capacity, size);
        } catch (error) {
          drop?.(step.value);
          throw error;
        }
      }
      length += 1;
      return step;
    },
    () => it.return?.(),
  );
  steps.from = [it];
  return steps;
}

// `it`, asked its hint before it's stepped, as std's `extend` of a new
// `String` or hash table asks it, and room reserved for its lower bound
// (string.rs:2507, hashbrown's `extend`): of a `String`'s bytes, no
// `align`, else of a table's entries of `size` bytes aligned to `align`.
// Where that throws, a panic, `it` is closed, as `extend` drops it (#733).
function $reserveHint(it, size, align) {
  try {
    const [lower] = $sizeHint(it);
    if (align === undefined) $roomFor(lower, size);
    else $tableRoomFor(lower, size, align);
  } catch (error) {
    it.return?.();
    throw error;
  }
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
