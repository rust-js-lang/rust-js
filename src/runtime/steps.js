// An iterator rust-js makes, stepped as Rust's is: each `next` steps on, so
// one that gave `None` may give more, as the crate's own or `from_fn`'s may,
// and its `map`, `filter`, `take` and `drop` are std's adapters, which pass
// that on, where JS's iterator helpers stay ended at their first `done`
// (#161, ADR 0669). Of a source that never gives more, each is JS's. `close`
// is what its `return()` closes.
class $Steps extends Iterator {
  constructor(next, close) {
    super();
    this.next = next;
    this.return = () => {
      close?.();
      return { done: true, value: undefined };
    };
  }
  map(f) {
    let i = 0;
    return new $Steps(
      () => {
        const step = this.next();
        return step.done ? step : { done: false, value: f(step.value, i++) };
      },
      () => this.return(),
    );
  }
  filter(keep) {
    let i = 0;
    return new $Steps(
      () => {
        for (;;) {
          const step = this.next();
          if (step.done || keep(step.value, i++)) return step;
        }
      },
      () => this.return(),
    );
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
  // Std's `Skip`: its first step passes `n`, or stops at a `None`, which it
  // gives; each after is its source's.
  drop(n) {
    return new $Steps(
      () => {
        for (; n > 0; n--) {
          const step = this.next();
          if (step.done) {
            n = 0;
            return step;
          }
        }
        return this.next();
      },
      () => this.return(),
    );
  }
}
