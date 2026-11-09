// `{:?}` of a `SystemTime` or an `Instant`, `name`, its `nanos`, as
// unix's and wasip1's std show it, its `Timespec`'s seconds and the
// nanoseconds past them, each an integer given the options:
// `SystemTime { tv_sec: 1, tv_nsec: 500000000 }` (ADR 0447).
function $timeDebug(name, nanos, options) {
  let seconds = nanos / 1000000000n;
  let past = nanos % 1000000000n;
  if (past < 0n) {
    seconds -= 1n;
    past += 1000000000n;
  }
  const shown = (n, bits) =>
    $formatted($debugInt(String(n), BigInt.asUintN(bits, n).toString(16), options), options, true);
  const fields = [`tv_sec: ${shown(seconds, 64)}`, `tv_nsec: ${shown(past, 32)}`];
  return options?.alternate ? $pretty(`${name} {`, fields, "}") : `${name} { ${fields.join(", ")} }`;
}
// `{:?}` of a `Duration` given a placeholder's options, as std's
// `fmt_decimal` applies them: `{:.3?}`'s digits, `+`'s sign, then its
// width, padded by the fill, to the left where it's not aligned, and never
// by zeros (ADR 0188).
function $debugDurationWith(duration, options) {
  let text = $debugDuration(duration, options?.precision);
  if (options?.plus) text = "+" + text;
  if (options?.width === undefined) return text;
  const align = { Left: "<", Right: ">", Center: "^" }[options.align] ?? "<";
  return $pad(text, options.width, align, options.fill);
}
// `{:?}` of a `SystemTimeError`, its `Duration`'s given the options, as
// std's derived `debug_tuple` gives them (ADR 0447).
function $systemTimeErrorDebug(duration, options) {
  const shown = $debugDurationWith(duration, options);
  return options?.alternate ? $pretty("SystemTimeError(", [shown], ")") : `SystemTimeError(${shown})`;
}
