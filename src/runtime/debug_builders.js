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
//
// Each step is std's `field` of its text, `show()`, or its `field_with`, of
// a value and a writing function, `write(g, value)`, that writes it into
// `g`, a piece at a time: `f`, or, for `{:#?}`, std's `PadAdapter` over it, `$padAdapter`.
// A writer shown into a writer writes each of its pieces so, each a call of
// the writer's (#1652, ADR 1135).
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
      // What's written ahead of its text pads it as it will be (#1490).
      if (pretty) $padAt.set(depth, adapter);
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
      } finally {
        if (pretty) $padAt.delete(depth);
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
// std's `PadAdapter` over the sink `f`, as `{:#?}` writes a part through it:
// each line a `write_str` of its own, the four spaces before one that starts
// a line another, before each of its lines, a blank line's too, and nothing
// of an empty one; a `char` a `write_char`, the spaces before it where it
// starts a line. `state`, whether it's at a line's start, is the part's.
function $padAdapter(f, state = { onNewline: true }) {
  return $sinkOf(
    (s) => {
      for (const line of s.split(/(?<=\n)/)) {
        if (line === "") continue;
        if (state.onNewline) f.write("    ");
        state.onNewline = line.endsWith("\n");
        f.write(line);
      }
    },
    () => "",
    (c) => {
      if (state.onNewline) f.write("    ");
      state.onNewline = c === "\n";
      f.writeChar(c);
    },
  );
}
// A list's or a set's `entry` and `entries`, std's `DebugInner`.
function $debugInner(f, pretty, close) {
  const inner = $debugBuilder(f, pretty);
  // `entry_with`: `value` written by `write(g, value)`.
  const entryWith = (value, write) => {
    inner.step(() => {
      if (pretty) {
        if (inner.fields === 0) f.write("\n");
        const g = $padAdapter(f);
        write(g, value);
        g.write(",\n");
      } else {
        if (inner.fields > 0) f.write(", ");
        write(f, value);
      }
    });
    inner.fields++;
  };
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
    entryWith,
    // Each of `items` stepped, written by `write(g, item)`, then dropped by
    // `drop`, as `entries` steps them.
    entriesWith(items, write, drop) {
      $eachOf(items, (item) => {
        if (drop === undefined) entryWith(item, write);
        else $unwinds(() => entryWith(item, write), () => drop(item));
        drop?.(item);
      });
    },
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
          $padAdapter(f).write("..\n");
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
  // `key_with` and `value_with`: `key` written by `writeKey(g, key)`, then
  // `: `, then `value` by `writeValue(g, value)`, one adapter's for `{:#?}`,
  // the value's not written where the key failed.
  const entryWith = (key, value, writeKey, writeValue) => {
    map.step(() => {
      if (pretty) {
        if (map.fields === 0) f.write("\n");
        const g = $padAdapter(f);
        writeKey(g, key);
        g.write(": ");
        writeValue(g, value);
        g.write(",\n");
      } else {
        if (map.fields > 0) f.write(", ");
        writeKey(f, key);
        f.write(": ");
        writeValue(f, value);
      }
    });
    map.fields++;
  };
  return {
    entry,
    entryWith,
    // Each of `items`, a `[key, value]` pair, written so.
    entriesWith(items, writeKey, writeValue, drop) {
      $eachOf(items, (item) => {
        const write = () => entryWith(item[0], item[1], writeKey, writeValue);
        if (drop === undefined) write();
        else $unwinds(write, () => drop(item));
        drop?.(item);
      });
    },
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
          $padAdapter(f).write("..\n");
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
    // `field_with`: its name, `: ` and its value, `write(g, value)`, each a
    // write.
    fieldWith(name, value, write) {
      struct.step(() => {
        if (pretty) {
          if (struct.fields === 0) f.write(" {\n");
          const g = $padAdapter(f);
          g.write(name);
          g.write(": ");
          write(g, value);
          g.write(",\n");
        } else {
          f.write(struct.fields > 0 ? ", " : " { ");
          f.write(name);
          f.write(": ");
          write(f, value);
        }
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
          $padAdapter(f).write("..\n");
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
    // `field_with`: `value`, written by `write(g, value)`.
    fieldWith(value, write) {
      tuple.step(() => {
        if (pretty) {
          if (tuple.fields === 0) f.write("(\n");
          const g = $padAdapter(f);
          write(g, value);
          g.write(",\n");
        } else {
          f.write(tuple.fields > 0 ? ", " : "(");
          write(f, value);
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
          $padAdapter(f).write("..\n");
          f.write(")");
        }
      });
      tuple.finish(() => {});
    },
  };
}
// A std value's `Debug`, `Some(x)`'s or a `Vec`'s, of a part that may stop
// part-way, `write(f)` of std's builders through a sink of its own, `f`:
// its text; or, where a part gave `Err`, that error, what was written
// before it carried out with it, as a writer's is (#1561).
function $composed(write) {
  const f = $sink();
  try {
    write(f);
  } catch (error) {
    if (error instanceof $FmtError) throw $fmtWritten(error, f.done());
    throw error;
  }
  return f.done();
}
// A value's `Debug` into a writer, by its builders, each part written by
// a writing function, `write(g, value)` (#1652): a sequence's list of its
// items; a tuple's fields, each of its `value`, `name` `""`, and a
// variant's, `Some`'s, `Ok`'s or `Err`'s; an `Option`'s `None`; a derived
// `Debug`'s struct, `fields` its `[name, value, write]`s, or tuple, its
// `[value, write]`s.
function $writeDebugList(f, items, write, pretty) {
  const list = $debugList(f, pretty);
  list.entriesWith(items, write);
  list.finish();
}
function $writeDebugFields(f, value, name, fields, pretty) {
  const tuple = $debugTuple(f, name, pretty);
  for (const write of fields) tuple.fieldWith(value, write);
  tuple.finish();
}
function $writeDebugStruct(f, name, fields, pretty) {
  const struct = $debugStruct(f, name, pretty);
  for (const [field, value, write] of fields) struct.fieldWith(field, value, write);
  struct.finish();
}
function $writeDebugTuple(f, name, fields, pretty) {
  const tuple = $debugTuple(f, name, pretty);
  for (const [value, write] of fields) tuple.fieldWith(value, write);
  tuple.finish();
}
function $writeDebugOption(f, option, write, pretty) {
  if (option == null) return f.write("None");
  $writeDebugFields(f, option, "Some", [write], pretty);
}
function $writeDebugResult(f, result, ok, err, pretty) {
  if (result.TAG === "Ok") $writeDebugFields(f, result, "Ok", [ok], pretty);
  else $writeDebugFields(f, result, "Err", [err], pretty);
}
