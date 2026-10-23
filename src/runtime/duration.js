// A `Duration` is its nanoseconds, a BigInt (ADR 0188): std's `MAX` is
// `u64::MAX` seconds and 999,999,999 nanoseconds, and past it, std panics.
function $durationNew(secs, nanos) {
  const duration = secs * 1000000000n + BigInt(nanos);
  if (duration > 18446744073709551615999999999n) throw new Error("overflow in Duration::new");
  return duration;
}
function $durationAdd(a, b) {
  const sum = a + b;
  if (sum > 18446744073709551615999999999n) throw new Error("overflow when adding durations");
  return sum;
}
function $durationSub(a, b) {
  if (a < b) throw new Error("overflow when subtracting durations");
  return a - b;
}
// `checked_add`'s: `None` past `MAX`.
function $durationChecked(sum) {
  return sum > 18446744073709551615999999999n ? undefined : sum;
}
// `{:?}`: in the largest unit it has a whole one of, its fraction's
// trailing zeros dropped, as std's writes it; with a `precision`, `{:.2?}`,
// that many digits of it, rounded to the nearest, a tie to even, as std's
// is, and zeros past its 9.
function $debugDuration(duration, precision) {
  const secs = duration / 1000000000n;
  const nanos = Number(duration % 1000000000n);
  const [whole, part, digits, unit] =
    secs > 0n
      ? [secs, nanos, 9, "s"]
      : nanos >= 1000000
        ? [Math.floor(nanos / 1000000), nanos % 1000000, 6, "ms"]
        : nanos >= 1000
          ? [Math.floor(nanos / 1000), nanos % 1000, 3, "µs"]
          : [nanos, 0, 0, "ns"];
  if (precision === undefined) {
    const fraction = part === 0 ? "" : "." + String(part).padStart(digits, "0").replace(/0+$/, "");
    return `${whole}${fraction}${unit}`;
  }
  const all = digits === 0 ? "" : String(part).padStart(digits, "0");
  let kept = all.slice(0, Math.min(precision, digits));
  let integer = BigInt(whole);
  const rest = all.slice(kept.length);
  const last = kept === "" ? Number(integer % 10n) : Number(kept[kept.length - 1]);
  // Up past half, and of a tie, to even.
  const half = "5".padEnd(rest.length, "0");
  if (rest !== "" && (rest > half || (rest === half && last % 2 === 1))) {
    const next = kept === "" ? "1" : String(BigInt(kept) + 1n).padStart(kept.length, "0");
    if (next.length > kept.length) {
      integer += 1n;
      kept = next.slice(1);
    } else kept = next;
  }
  return `${integer}${precision > 0 ? "." + kept.padEnd(precision, "0") : ""}${unit}`;
}
