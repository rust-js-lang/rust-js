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
