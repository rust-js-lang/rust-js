// A binary's start, std's `lang_start` (#2292): its `main` run, `entry`,
// the status it gives, `0` of one that gives nothing, or `101` of a panic,
// and the process ended with a failing status at once, as std's exit ends
// it: nothing runs after, a binding's pending timer neither. What it wrote
// is all written first: Node writes a pipe later on macOS, which an exit
// would cut off, so the process's streams are made to write at once as it
// starts, where Node lets them be. A file, and each of Bun's streams, is
// written as it's given, `_type` `"fs"`, so the process ends at once there
// too: waiting for its write to call back ran what `main` queued, a
// promise's reaction, where native ends at the panic (#2538). Another
// stream is ended once each has written what it holds. A status of `0`
// lets the process end as it does by
// itself. In a browser, which has no process, a panic is thrown on to the
// page, and a failing status thrown as an error, its signal of a failure
// (#847's of an abort). What rust-js doesn't support as the program runs
// (`$unsupported`) is said as its refusals at compile time are, `error:
// rust-js does not support ..` and where the program asked it, status 1,
// never Node's report of an uncaught error (#2447, #2493).
function $main(entry) {
  const process = globalThis.process;
  const browser = process?.argv === undefined;
  if (!browser) for (const stream of [process.stdout, process.stderr]) stream?._handle?.setBlocking?.(true);
  if (!browser) globalThis[$hookKey] = $hook;
  let status;
  try {
    status = entry() ?? 0;
  } catch (thrown) {
    // As a scope's cleanup gives it on (`$unwindsOn`): a stack overflow is
    // Rust's abort, its lines and exit 134, never a panic (#2452).
    const e = $unwindsOn(thrown);
    if (!browser && e instanceof $Unsupported) {
      $eprint($refusedAsItRan(e));
      status = 1;
    } else if (browser || $endsTheProgram(e)) {
      throw e;
    } else {
      $hook(e);
      status = 101;
    }
  }
  if (browser) {
    // its own Error: a browser's signal of a failing status, which has no process to end.
    if (status !== 0) throw new Error(`main exited with status ${status}`);
    return;
  }
  if (status === 0) return;
  process.exitCode = status & 255;
  const blocking = [process.stdout, process.stderr].every(
    (stream) => stream?._handle?.setBlocking !== undefined || stream?.isTTY || stream?._type === "fs",
  );
  if (blocking) process.exit(status & 255);
  process.stdout.write("", () => process.stderr.write("", () => process.exit(status & 255)));
}

// A refusal as the program ran, `e`, as rust-js writes one at compile
// time: its message, and where it was asked, the first frame of its stack
// outside the runtime's own module, `$unsupported`'s, the JS's place, as
// the engine gives it; none where every frame is the runtime's, a bundle's.
function $refusedAsItRan(e) {
  const places = String(e.stack ?? "")
    .split("\n")
    .map((line) => /^\s*at (?:.* \()?(.+?):(\d+):(\d+)\)?$/.exec(line))
    .filter((place) => place !== null);
  const own = places[0]?.[1];
  const asked = places.find((place) => place[1] !== own);
  const where = asked === undefined ? "" : ` --> ${asked[1].replace(/^file:\/\//, "")}:${asked[2]}:${asked[3]}\n`;
  return `error: ${e.message}\n${where}`;
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
