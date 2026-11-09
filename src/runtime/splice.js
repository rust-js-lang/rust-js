// `v.splice(start..end, replacement)`, std's `Splice` (ADR 0501): what the
// range had taken out, an iterator over it that owns each, `dropItem`'s;
// `items` left its head and its tail kept apart. Where it's done with,
// dropped or stepped to its end, its close runs std's `Splice::drop`: what
// it hasn't given dropped, then `replacement` stepped into the gap, each
// put in place as it's stepped, so what's put stays where a step throws, a
// panic; then as many more as its hint's lower bound; then what's left
// collected apart, dropped where a step throws, and put in place. Then, as
// it ends or unwinds, the tail put back, `Drain::drop`'s, and what's left
// of `replacement` closed, the `Splice`'s fields in their order (#1147,
// #1149).
function $splice(items, start, end, replacement, dropItem) {
  $checkRange(start, end, items.length);
  const tail = items.splice(end);
  const removed = items.splice(start);
  const gap = removed.length;
  const it = typeof replacement.next === "function" ? replacement : replacement[Symbol.iterator]();
  // At most `n` of it put in place, `Drain::fill`'s: whether it has more.
  const fill = (n) => {
    for (let i = 0; i < n; i++) {
      const step = it.next();
      if (step.done) return false;
      items.push(step.value);
    }
    return true;
  };
  const filled = () => {
    // Of no tail, `vec.extend(replacement)`: each pushed as it's stepped.
    if (tail.length === 0) return void fill(Infinity);
    if (!fill(gap)) return;
    const [lower] = $sizeHint(it);
    if (lower > 0 && !fill(lower)) return;
    const collected = [];
    $unwinds(
      () => {
        for (let step = it.next(); !step.done; step = it.next()) collected.push(step.value);
      },
      dropItem && (() => $dropItems(collected, dropItem)),
    );
    $extend(items, collected);
  };
  // The tail put back, then `replacement` closed, the rest of the
  // `Splice`'s drop, at its end and as a panic unwinds.
  const restored = () => $extend(items, tail);
  const closed = () => it.return?.();
  return $closing($iter(removed), (rest) => {
    $unwinds(
      () => {
        if (dropItem !== undefined) $dropItems(rest, dropItem);
        filled();
      },
      restored,
      closed,
    );
    restored();
    closed();
  });
}
