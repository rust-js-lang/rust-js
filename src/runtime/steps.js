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
//
// Its `tryFold` is std's `try_fold`, which no crate's iterator writes (its
// `Try` is unstable), but std's adapters do: what stops early, `some`
// (`any`), `every` (`all`), `find`, `$position` and `try_fold`'s helpers,
// asks it, and a `map`, a `take` and a `flatMap` pass it to their source,
// and a `filter` steps its source's `find`, as std's do, so a `drop`'s is
// std's `Skip`'s: its source's `nth(n - 1)`, then its source's `tryFold`.
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
  // Std's `try_fold`: `f` of each item, given what it gave, until `broke`
  // says what it gave stops it, as `?` would, which it gives back.
  tryFold(init, f, broke) {
    let folded = init;
    for (let step = this.next(); !step.done; step = this.next()) {
      folded = f(folded, step.value);
      if (broke(folded)) return folded;
    }
    return folded;
  }
  some(found) {
    let i = 0;
    const some = this.tryFold(false, (_, x) => found(x, i++), (hit) => hit);
    if (some) this.return();
    return some;
  }
  every(holds) {
    let i = 0;
    const every = this.tryFold(true, (_, x) => holds(x, i++), (held) => !held);
    if (!every) this.return();
    return every;
  }
  find(found) {
    let i = 0;
    let item;
    const hit = this.tryFold(false, (_, x) => found(x, i++) && ((item = x), true), (hit) => hit);
    if (hit) this.return();
    return item;
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
    mapped.tryFold = (init, g, broke) => this.tryFold(init, (a, x) => g(a, f(x, i++)), broke);
    // Std's `Map::size_hint`, its source's.
    mapped.hint = () => $sizeHint(this);
    return mapped;
  }
  filter(keep) {
    let i = 0;
    // Each step, std's `Filter::next`, its source's `find`, a `try_fold`.
    const kept = new $Steps(
      () => {
        let item;
        const found = this.tryFold(false, (_, x) => keep(x, i++) && ((item = x), true), (found) => found);
        return found ? { done: false, value: item } : { done: true, value: undefined };
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
    // Its `try_fold`, std's: the rest of the one it's in, then its source's
    // of each one's, the one it stops in kept for its next step.
    flat.tryFold = (init, g, broke) => {
      let folded = init;
      if (inner !== undefined) {
        folded = $lent(inner).tryFold(folded, g, broke);
        if (broke(folded)) return folded;
        inner = undefined;
      }
      if (ended) return folded;
      folded = this.tryFold(
        folded,
        (a, x) => {
          inner = Iterator.from(f(x, i++));
          const made = $lent(inner).tryFold(a, g, broke);
          if (!broke(made)) inner = undefined;
          return made;
        },
        broke,
      );
      if (!broke(folded)) ended = true;
      return folded;
    };
    return flat;
  }
  // Std's `Take`: each of its `n` steps steps its source, a `None` too.
  take(n) {
    const taken = new $Steps(
      () => {
        if (n === 0) return this.return();
        n--;
        return this.next();
      },
      () => this.return(),
    );
    // Its `nth(k)`, std's: its source's, where it has more than `k` left;
    // else its source's past what it has left, and done.
    taken.nth = (k) => {
      if (n > k) {
        n -= k + 1;
        return this.nth(k);
      }
      if (n > 0) this.nth(n - 1);
      n = 0;
      return this.return();
    };
    // Its `try_fold`, std's: its source's, stopped once it's taken `n`.
    taken.tryFold = (init, g, broke) => {
      if (n === 0) return init;
      let full = false;
      return this.tryFold(
        init,
        (a, x) => {
          n--;
          full = n === 0;
          return g(a, x);
        },
        (a) => full || broke(a),
      );
    };
    return taken;
  }
  // Std's `Skip`: its first step is its source's `nth(n)`, which stops at a
  // `None`, and gives it; each after is its source's. Its fold and its
  // `try_fold`, the source's after its `nth(n - 1)`; its `nth(k)`, its
  // source's `nth(n + k)`.
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
    skipped.tryFold = (init, g, broke) => {
      if (n > 0) {
        const k = n;
        n = 0;
        if (this.nth(k - 1).done) return init;
      }
      return this.tryFold(init, g, broke);
    };
    skipped.nth = (k) => {
      const skip = n;
      n = 0;
      return this.nth(skip + k);
    };
    return skipped;
  }
}
