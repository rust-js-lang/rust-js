// `a.chain(b)`, lazy: each of `a`'s, then each of `b`'s; both closed as it
// ends, which drops what each holds (ADR 0465).
function* $lazyChain(a, b) {
  try {
    yield* a;
    yield* b;
  } finally {
    a.return?.();
    b.return?.();
  }
}
