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
  // Held, `$held`'s: what its owner drops where Rust drops it, a variable
  // at its scope's end, a temporary at its statement's (#206). What it steps
  // is then dropped only as it's closed, not as it's done or ends early, as
  // each adapter it steps is held too: its sources, `from`, or what it
  // steps, `iter`.
  hold() {
    if (this.held) return;
    this.held = true;
    for (const source of this.from ?? (this.iter === undefined ? [] : [this.iter])) source.hold?.();
  }
  // Std's `fold`: `f` of each item, given what it gave. Consuming it, it
  // closes it, as Rust's `fold` drops what it takes at its end: what a held
  // source has left, one a variable moved here held (#206).
  fold(init, f) {
    let folded = init;
    for (let step = this.next(); !step.done; step = this.next()) folded = f(folded, step.value);
    this.return();
    return folded;
  }
  // `collect()`'s, each item, then closed, as `fold` is.
  toArray() {
    const items = [];
    for (let step = this.next(); !step.done; step = this.next()) items.push(step.value);
    this.return();
    return items;
  }
  // Std's `last`: the last item its `fold` gives, a step, done where there's
  // none; an adapter whose `last` is its source's, `skip`'s, a chain's, a
  // fuse's or a peekable's, passes it on to the crate's own (#182).
  last() {
    let found = false;
    let value;
    this.fold(undefined, (_, x) => {
      found = true;
      value = x;
    });
    return found ? { done: false, value } : { done: true, value: undefined };
  }
  // Std's `nth`: `advance_by(k)`, a `None` among them ending it, then the
  // next, so an adapter's reaches its source's `try_fold` (#214).
  nth(k, drop) {
    if (this.advanceBy(k, drop) > 0) return { done: true, value: undefined };
    return this.next();
  }
  // Std's `advance_by(m)`, of a sized iterator: `m` stepped past by its
  // `try_fold`, as std's default is, each given to `drop` where it has one;
  // how many of them there weren't (#214).
  advanceBy(m, drop) {
    if (m === 0) return 0;
    return this.tryFold(
      m,
      (k, x) => {
        drop?.(x);
        return k - 1;
      },
      (k) => k === 0,
    );
  }
  // Std's `count`: of its `fold`, each item given to `drop` where it has one.
  count(drop) {
    return this.fold(0, (n, x) => {
      drop?.(x);
      return n + 1;
    });
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
    mapped.from = [this];
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
    kept.from = [this];
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
    flat.from = [this];
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
        // At its limit, what it steps is done with, unless it's held,
        // whose owner drops it (ADR 0570, #206).
        if (n === 0) return taken.held ? { done: true, value: undefined } : this.return();
        n--;
        return this.next();
      },
      () => this.return(),
    );
    taken.from = [this];
    // Its `nth` drops what it passes over itself where its source's does, a
    // reversed `Vec`'s, as one slice, by each item's drop, its source's or
    // the one it's given (#261, #286).
    if (this.ownNth) {
      taken.ownNth = true;
      if (this.dropItem !== undefined) taken.dropItem = this.dropItem;
    }
    // Its `nth(k)`, std's: its source's, where it has more than `k` left;
    // else its source's past what it has left, and done.
    taken.nth = (k, drop = taken.dropItem) => {
      if (n > k) {
        n -= k + 1;
        return this.nth(k, drop);
      }
      // Past its end, its source's `nth(n - 1)`, whose item std's `Take::nth`
      // drops, where its source says how (#261).
      if (n > 0) {
        const step = this.nth(n - 1, drop);
        if (!step.done) drop?.(step.value);
      }
      n = 0;
      // Past its end, what it steps done with, unless it's held, whose
      // owner drops it (#242).
      return taken.held ? { done: true, value: undefined } : this.return();
    };
    // Its `try_fold`, std's: its source's, stopped once it's taken `n`;
    // asked again there, or stopped there by its limit, not by what it
    // gave, which its caller still steps, its source done with, as its
    // `next` closes it (ADR 0570, #225, #242).
    taken.tryFold = (init, g, broke) => {
      if (n === 0) {
        if (!taken.held) this.return();
        return init;
      }
      let full = false;
      let stopped = false;
      const folded = this.tryFold(
        init,
        (a, x) => {
          n--;
          full = n === 0;
          return g(a, x);
        },
        (a) => (stopped = broke(a)) || full,
      );
      if (full && !stopped && !taken.held) this.return();
      return folded;
    };
    // Its `fold`, std's: its `try_fold`, never stopped, so a `Skip`'s
    // inside asks its source's `nth(n - 1)`, then its source dropped with it
    // (#213).
    taken.fold = (init, g) => {
      const folded = taken.tryFold(init, g, () => false);
      this.return();
      return folded;
    };
    // Its `advance_by(m)` and `count`, std's: its source advanced, by as
    // many as it has left, which asks no `nth` (#213).
    taken.advanceBy = (m, drop) => {
      const min = Math.min(n, m);
      const advanced = min - this.advanceBy(min, drop);
      n -= advanced;
      return m - advanced;
    };
    taken.count = (drop) => {
      const k = n;
      n = 0;
      const counted = k === 0 ? 0 : k - this.advanceBy(k, drop);
      this.return();
      return counted;
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
    skipped.from = [this];
    // Its `fold`, then closed where its `nth(n - 1)` found none, as
    // `fold(self)` drops what it has left at its end (#242).
    skipped.fold = (init, g) => {
      if (n > 0) {
        const k = n;
        n = 0;
        if (this.nth(k - 1).done) {
          this.return();
          return init;
        }
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
    // Its `advance_by(m)`, std's: its source advanced past what it skips
    // and `m` more (#213).
    skipped.advanceBy = (m, drop) => {
      const skip = n;
      const left = this.advanceBy(skip + m, drop);
      const advanced = skip + m - left;
      n = Math.max(skip - advanced, 0);
      return m - Math.max(advanced - skip, 0);
    };
    // Std's `Skip::last`: its source's `nth(n - 1)`, then its source's `last`.
    skipped.last = () => {
      if (n > 0) {
        const k = n;
        n = 0;
        if (this.nth(k - 1).done) return { done: true, value: undefined };
      }
      return this.last();
    };
    return skipped;
  }
}
