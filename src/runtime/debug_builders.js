// std's `Debug` builders, `f.debug_list()` and the rest, of a writer that
// writes through stdout's line, its sink `f` (#1490): core's
// `fmt/builders.rs`, step for step, each piece written as it's taken (#1533).
// The opening when the builder's made, `[` or the type's name; each step's
// separator, then its text, `show()`, made only then; the close at `finish`.
// So what the writer prints between them, as an `entries` iterator steps,
// as a field's value is made, or as an entry drops, lands where Rust's
// stdout puts it, `[<step 1>1<step 2>, 2]`. `{:#?}`'s (`pretty`) each part
// is written through std's `PadAdapter`, `$padded`, its state kept between a
// field's name and its value. A step's `Err`, a write's error a `show`
// threw, is the builder's `result`, as std's: what the part wrote before it
// is written, and no later step is taken, its `show` never made, though
// `entries` still steps its iterator; `finish` throws it. A panic goes on,
// what the part wrote before it written as the builder's, through the
// adapter, as std's `PadAdapter` wrote it.
function $debugBuilder(f, pretty) {
  const builder = {
    // std's `result`: `undefined`, `Ok`, or the error a step gave.
    result: undefined,
    fields: 0,
    // A step of std's `and_then`: `write`, given the adapter its part is
    // written through, a fresh one's for `{:#?}`.
    step(write) {
      if (builder.result !== undefined) return;
      const adapter = { onNewline: true };
      const written = (text) => f.write(pretty ? $padded(text, adapter) : text);
      const depth = $captures.length;
      try {
        write(written);
      } catch (error) {
        if (!(error instanceof $WriteError)) {
          const unfinished = $unfinishedAbove(depth);
          if (unfinished !== "") written(unfinished);
          throw error;
        }
        // What the part wrote before its error, in the writer's text, which
        // the error then carries no more of.
        if (error.written) written(error.written);
        error.written = "";
        builder.result = error;
      }
    },
    // The close, `close()`, unless a step failed; then its error.
    finish(close) {
      if (builder.result === undefined) close();
      if (builder.result !== undefined) throw builder.result;
    },
  };
  return builder;
}
// A list's or a set's `entry` and `entries`, std's `DebugInner`.
function $debugInner(f, pretty, close) {
  const inner = $debugBuilder(f, pretty);
  const entry = (show) => {
    inner.step((written) => {
      if (pretty) {
        if (inner.fields === 0) f.write("\n");
        written(show());
        written(",\n");
      } else {
        if (inner.fields > 0) f.write(", ");
        f.write(show());
      }
    });
    inner.fields++;
  };
  return {
    entry,
    // Each of `items` stepped, shown, then dropped by `drop`, as std's `for
    // entry in entries { self.entry(&entry) }` drops it, and as a panic
    // showing it unwinds; what's left of `items` closed where a step or a
    // show panics (`$entriesShown`'s).
    entries(items, show, drop) {
      $eachOf(items, (item) => {
        if (drop === undefined) entry(() => show(item));
        else $unwinds(() => entry(() => show(item)), () => drop(item));
        drop?.(item);
      });
    },
    finish: () => inner.finish(() => f.write(close)),
    finishNonExhaustive() {
      inner.step((written) => {
        if (inner.fields === 0) f.write(`..${close}`);
        else if (!pretty) f.write(`, ..${close}`);
        else {
          written("..\n");
          f.write(close);
        }
      });
      inner.finish(() => {});
    },
  };
}
// `f.debug_list()`: `[`, then its entries.
function $debugList(f, pretty) {
  f.write("[");
  return $debugInner(f, pretty, "]");
}
// `f.debug_set()`: `{`, then its entries.
function $debugSet(f, pretty) {
  f.write("{");
  return $debugInner(f, pretty, "}");
}
// `f.debug_map()`: `{`, then each entry's key, `: ` and value, `showKey()`
// then `showValue()`, the value's not made where the key failed.
function $debugMap(f, pretty) {
  f.write("{");
  const map = $debugBuilder(f, pretty);
  const entry = (showKey, showValue) => {
    map.step((written) => {
      if (pretty && map.fields === 0) f.write("\n");
      if (!pretty && map.fields > 0) f.write(", ");
      written(showKey());
      written(": ");
      written(showValue());
      if (pretty) written(",\n");
    });
    map.fields++;
  };
  return {
    entry,
    entries(items, showKey, showValue, drop) {
      $eachOf(items, (item) => {
        const show = () => entry(() => showKey(item), () => showValue(item));
        if (drop === undefined) show();
        else $unwinds(show, () => drop(item));
        drop?.(item);
      });
    },
    finish: () => map.finish(() => f.write("}")),
    finishNonExhaustive() {
      map.step((written) => {
        if (map.fields === 0) f.write("..}");
        else if (!pretty) f.write(", ..}");
        else {
          written("..\n");
          f.write("}");
        }
      });
      map.finish(() => {});
    },
  };
}
// `f.debug_struct(name)`: its name, then each field's `name: value`, in
// braces where it has any.
function $debugStruct(f, name, pretty) {
  f.write(name);
  const struct = $debugBuilder(f, pretty);
  return {
    field(name, show) {
      struct.step((written) => {
        if (pretty) {
          if (struct.fields === 0) f.write(" {\n");
        } else f.write(struct.fields > 0 ? ", " : " { ");
        written(`${name}: `);
        written(show());
        if (pretty) written(",\n");
      });
      struct.fields++;
    },
    finish: () =>
      struct.finish(() => {
        if (struct.fields > 0) f.write(pretty ? "}" : " }");
      }),
    finishNonExhaustive() {
      struct.step((written) => {
        if (struct.fields === 0) f.write(" { .. }");
        else if (!pretty) f.write(", .. }");
        else {
          written("..\n");
          f.write("}");
        }
      });
      struct.finish(() => {});
    },
  };
}
// `f.debug_tuple(name)`: its name, then its fields in parentheses where it
// has any, one of no name's with a comma after it, `(1,)`.
function $debugTuple(f, name, pretty) {
  f.write(name);
  const tuple = $debugBuilder(f, pretty);
  return {
    field(show) {
      tuple.step((written) => {
        if (pretty) {
          if (tuple.fields === 0) f.write("(\n");
          written(show());
          written(",\n");
        } else {
          f.write(tuple.fields > 0 ? ", " : "(");
          f.write(show());
        }
      });
      tuple.fields++;
    },
    finish: () =>
      tuple.finish(() => {
        if (tuple.fields === 0) return;
        if (tuple.fields === 1 && name === "" && !pretty) f.write(",");
        f.write(")");
      }),
    finishNonExhaustive() {
      tuple.step((written) => {
        if (tuple.fields === 0) f.write("(..)");
        else if (!pretty) f.write(", ..)");
        else {
          written("..\n");
          f.write(")");
        }
      });
      tuple.finish(() => {});
    },
  };
}
