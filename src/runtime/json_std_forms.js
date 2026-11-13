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
