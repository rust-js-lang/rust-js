// An iterator rust-js makes, stepped as Rust's is: each `next` steps on, so
// one that gave `None` may give more, as the crate's own or `from_fn`'s may,
// and its `map`, `filter`, `flatMap`, `take` and `drop` are std's adapters, which pass
// that on, where JS's iterator helpers stay ended at their first `done`
// (#161, ADR 0669). Of a source that never gives more, each is JS's. `close`
// is what its `return()` closes.
//
// Its `fold` and `nth` are std's, of its `next`, unless the crate's iterator
// it steps writes its own, which `$iterator` gives it: then what std routes
// through them does too, `reduce` (`sum`, `count`, `fold`), `forEach`, a
// `map`'s, a `filter`'s or a `flatMap`'s fold, and `drop`'s (`skip`'s)
// first step (#162).
class $Steps extends Iterator {
  constructor(next, close) {
    super();
    this.next = next;
    this.return = () => {
      close?.();
      return { done: true, value: undefined };
    };
  }
  // Std's `fold`: `f` of each item, given what it gave.
  fold(init, f) {
    let folded = init;
    for (let step = this.next(); !step.done; step = this.next()) folded = f(folded, step.value);
    return folded;
  }
  // Std's `nth`: `k` stepped past, a `None` among them ending it, then the next.
  nth(k) {
    for (; k > 0; k--) {
      const step = this.next();
      if (step.done) return step;
    }
    return this.next();
  }
  reduce(f, ...init) {
    let i = 0;
    if (init.length > 0) return this.fold(init[0], (a, x) => f(a, x, i++));
    const first = this.next();
    if (first.done) throw new TypeError("Reduce of empty iterator with no initial value");
    i = 1;
    return this.fold(first.value, (a, x) => f(a, x, i++));
  }
  forEach(f) {
    let i = 0;
    this.fold(undefined, (_, x) => {
      f(x, i++);
    });
  }
  map(f) {
    let i = 0;
    const mapped = new $Steps(
      () => {
        const step = this.next();
        return step.done ? step : { done: false, value: f(step.value, i++) };
      },
      () => this.return(),
    );
    // Std's `Map::fold`, its source's of each mapped.
    mapped.fold = (init, g) => this.fold(init, (a, x) => g(a, f(x, i++)));
    return mapped;
  }
  filter(keep) {
    let i = 0;
    const kept = new $Steps(
      () => {
        for (;;) {
          const step = this.next();
          if (step.done || keep(step.value, i++)) return step;
        }
      },
      () => this.return(),
    );
    // Std's `Filter::fold`, its source's of each kept.
    kept.fold = (init, g) => this.fold(init, (a, x) => (keep(x, i++) ? g(a, x) : a));
    return kept;
  }
  // Std's `FlatMap`: each item of what `f` makes of each of its source's,
  // its source stepped no more once it's given `None`, as std's fuses it.
  // Its fold, std's, the rest of the one it's in, then its source's of each
  // one's fold.
  flatMap(f) {
    let i = 0;
    let inner;
    let ended = false;
    const flat = new $Steps(
      () => {
        for (;;) {
          if (inner !== undefined) {
            const step = inner.next();
            if (!step.done) return step;
            inner = undefined;
          }
          if (ended) return { done: true, value: undefined };
          const step = this.next();
          if (step.done) {
            ended = true;
            return step;
          }
          inner = Iterator.from(f(step.value, i++));
        }
      },
      () => {
        inner?.return?.();
        this.return();
      },
    );
    flat.fold = (init, g) => {
      let folded = init;
      if (inner !== undefined) folded = inner.reduce(g, folded);
      inner = undefined;
      if (ended) return folded;
      ended = true;
      return this.fold(folded, (a, x) => Iterator.from(f(x, i++)).reduce(g, a));
    };
    return flat;
  }
  // Std's `Take`: each of its `n` steps steps its source, a `None` too.
  take(n) {
    return new $Steps(
      () => {
        if (n === 0) return this.return();
        n--;
        return this.next();
      },
      () => this.return(),
    );
  }
  // Std's `Skip`: its first step is its source's `nth(n)`, which stops at a
  // `None`, and gives it; each after is its source's. Its fold, the source's
  // after its `nth(n - 1)`.
  drop(n) {
    const skipped = new $Steps(
      () => {
        if (n === 0) return this.next();
        const k = n;
        n = 0;
        return this.nth(k);
      },
      () => this.return(),
    );
    skipped.fold = (init, g) => {
      if (n > 0) {
        const k = n;
        n = 0;
        if (this.nth(k - 1).done) return init;
      }
      return this.fold(init, g);
    };
    return skipped;
  }
}
