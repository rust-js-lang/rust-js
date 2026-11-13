// The checks of serde's own impls of std's types, a catalog row's (#1623,
// ADR 1134), each in serde's words.

// A `Duration` of its `secs` and `nanos`: serde's overflow, of the whole
// seconds its nanoseconds carry past `u64::MAX`, then `Duration::new`'s,
// which carries them.
function $durationOfJson(secs, nanos) {
  if (secs + BigInt(Math.floor(nanos / 1000000000)) > 18446744073709551615n) {
    throw $jsonError("overflow deserializing Duration");
  }
  return $durationNew(secs, nanos);
}

// A `SystemTime`'s time since the Unix epoch, `0n` (ADR 0447), which serde
// writes only of one after it.
function $systemTimeSinceEpochForJson(time) {
  if (time < 0n) throw $jsonError("SystemTime must be later than UNIX_EPOCH");
  return time;
}

// A `SystemTime`'s offset from the epoch, of its `secs_since_epoch` and
// `nanos_since_epoch`: serde's overflow, as a `Duration`'s, in its own words.
function $systemTimeOffsetOfJson(secs, nanos) {
  if (secs + BigInt(Math.floor(nanos / 1000000000)) > 18446744073709551615n) {
    throw $jsonError("overflow deserializing SystemTime epoch offset");
  }
  return $durationNew(secs, nanos);
}

// A `SystemTime` of its offset, read whole: `UNIX_EPOCH.checked_add`'s
// overflow, whose seconds unix's `SystemTime` holds as an `i64`, which
// serde checks past its reader, so its error has no position.
function $systemTimeOfJson(offset) {
  if (offset / 1000000000n > 9223372036854775807n) throw $jsonError("overflow deserializing SystemTime");
  return offset;
}

// A `RefCell`'s value written by `write` while it's borrowed, serde's
// `try_borrow()`: `already mutably borrowed` where it's mutably borrowed.
// Of one that's the object it holds (`itself`, ADR 0863), the cell.
function $jsonBorrowed(cell, write, itself) {
  if (cell.borrows < 0) throw $jsonError("already mutably borrowed");
  $withBorrow(cell, write, itself);
}

// A `Mutex`'s or an `RwLock`'s value written by `write` while it's locked,
// serde's `lock()`, or `read()` of an `RwLock` (not `exclusive`): `lock
// poison error while serializing` where a panic poisoned it; locked again
// while the thread holds it, rust-js's lock's own refusal of Rust's
// deadlock (ADR 0328). Its guard released as a panic unwinds out of
// `write`, by `$unwinds`, a `Mutex`'s poisoning it, as its guard dropped as
// a panic unwinds does (`$unlock`); a write's error, a `Result`
// (`$jsonTry`'s), released as any return is, and given on.
function $jsonLocked(cell, write, exclusive) {
  const locked = $locked(cell, exclusive);
  const release = exclusive ? () => $unlock(cell) : () => $unborrow(cell);
  if (locked.TAG === "Err") {
    release();
    throw $jsonError("lock poison error while serializing");
  }
  const written = $unwinds(() => $jsonTry(() => write(cell.value)), release);
  release();
  if (written.TAG === "Err") throw written._0;
}
