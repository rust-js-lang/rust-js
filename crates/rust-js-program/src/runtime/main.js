// A binary's start, std's `lang_start` (#2292): its `main` run, `entry`,
// the status it gives, `0` of one that gives nothing, or `101` of a panic,
// and the process ended with a failing status at once, as std's exit ends
// it: nothing runs after, a binding's pending timer neither. What it wrote
// is all written first: Node writes a pipe later on macOS, which an exit
// would cut off, so the process's streams are made to write at once as it
// starts, where Node lets them be, and else ended once each has written
// what it holds. A status of `0` lets the process end as it does by
// itself. In a browser, which has no process, a panic is thrown on to the
// page, and a failing status thrown as an error, its signal of a failure
// (#847's of an abort).
function $main(entry) {
  const process = globalThis.process;
  const browser = process?.argv === undefined;
  if (!browser) for (const stream of [process.stdout, process.stderr]) stream?._handle?.setBlocking?.(true);
  if (!browser) $onPanic = $hook;
  let status;
  try {
    status = entry() ?? 0;
  } catch (e) {
    if (browser || $endsTheProgram(e)) throw e;
    $hook(e);
    status = 101;
  }
  if (browser) {
    if (status !== 0) throw new Error(`main exited with status ${status}`);
    return;
  }
  if (status === 0) return;
  process.exitCode = status & 255;
  const blocking = [process.stdout, process.stderr].every((stream) => stream?._handle?.setBlocking !== undefined || stream?.isTTY);
  if (blocking) process.exit(status & 255);
  process.stdout.write("", () => process.stderr.write("", () => process.exit(status & 255)));
}

// std's panic hook's text of `e`, a panic, on stderr, once: where it was
// raised, as `$panic` marked it, and its message, as std 1.100's default
// hook writes them, a newline first, and of the process's first panic
// alone, the note of a backtrace (#2292). The thread is the process's.
// What JS threw, which reaches `main`, is written so too, raised at
// `<unknown>`; a `resume_unwind`'s never (`$unwinding`).
const $hooked = new WeakSet();
let $noted = false;
function $hook(e) {
  if (typeof e !== "object" || e === null || $hooked.has(e)) return;
  $hooked.add(e);
  const at = e[$raisedAt] ?? "<unknown>";
  const thread = globalThis.process?.pid ?? 0;
  const note = $noted ? "" : "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n";
  $noted = true;
  $eprint(`\nthread 'main' (${thread}) panicked at ${at}:\n${e.message}\n${note}`);
}
