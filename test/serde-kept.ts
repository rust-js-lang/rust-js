// serde's built crates as a compile reads them: a snapshot of what
// serde/build.sh's flags name, each file and directory copied, as a clone
// where the disk can, into a directory named by what they hold. A Cargo
// build of `target/serde` after, by another process, writes its own files
// anew, and never takes one from under a compile still reading it, which
// rustc says as "can't find crate for `serde`" (#1711).
import { createHash } from "node:crypto";
import { constants, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";

import { bytesOf, type Kept } from "../tooling/bounded.js";
import { alive } from "./kept";

/** The files of `path` a compile reads: itself, a file's, or a directory's
 * own files, as rustc's `-L dependency=` search reads it, none of its
 * subdirectories. Under Cargo 1.100 the directory is a build step's
 * `build/<package>/<hash>/out`, where proc-macro2's build script writes and
 * removes probe files, `out/probe`, while another run's Cargo builds: a
 * walk of it found one gone between listing it and reading it (#2311).
 * Listed by each entry's type, with no stat to race. */
function read(path: string): string[] {
  // A crate's metadata and what rust-js noted beside it, `<stem>.rust-js-*`,
  // which a compile reads with it: a registry crate's sites (#2325).
  if (!statSync(path).isDirectory()) {
    const stem = basename(path).replace(/\.rmeta$/, ".");
    const noted = path.endsWith(".rmeta")
      ? readdirSync(dirname(path)).filter((name) => name.startsWith(`${stem}rust-js-`)).sort().map((name) => join(dirname(path), name))
      : [];
    return [path, ...noted];
  }
  return (readdirSync(path, { withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => join(path, entry.name))
    .sort();
}

/** Whether `error` is of a file gone since it was listed. */
const gone = (error: unknown) => (error as NodeJS.ErrnoException).code === "ENOENT";

/** Each file of `path` a compile reads (`read`), whole, into `hash`, under
 * `as`. A directory's file gone since it was listed, an artifact Cargo let
 * go, which no flag names, is left out. */
function hashed(hash: ReturnType<typeof createHash>, path: string, as: string) {
  for (const file of read(path)) {
    let text: Buffer;
    try {
      text = readFileSync(file);
    } catch (error) {
      if (file !== path && gone(error)) continue;
      throw error;
    }
    hash.update(`${as}/${basename(file)}\0`).update(text).update("\0");
  }
}

/** Each file of `from` a compile reads (`read`) copied to `to`, a file's to
 * `to` itself, each a clone where the disk can: by `copy`, which a test
 * gives to take a file away as it's copied. A directory's file gone since
 * it was listed is left out, as `hashed` leaves it. */
export function copied(from: string, to: string, copy = (a: string, b: string) => copyFileSync(a, b, constants.COPYFILE_FICLONE)) {
  const files = read(from);
  const whole = files[0] === from;
  mkdirSync(whole ? dirname(to) : to, { recursive: true });
  // A file's, `to` itself, and what's noted beside it, beside `to` by its
  // stem: `libserde_json-1a.rust-js-sites` of `to`'s `libserde_json-2b.rmeta`.
  const placed = (file: string) =>
    file === from ? to : whole ? join(dirname(to), basename(file).replace(basename(from).replace(/\.rmeta$/, ""), basename(to).replace(/\.rmeta$/, ""))) : join(to, basename(file));
  for (const file of files) {
    try {
      copy(file, placed(file));
    } catch (error) {
      if (file !== from && gone(error)) continue;
      throw error;
    }
  }
}

/** How long a snapshot no live run reads is kept after it was last asked
 * for: an hour. A run reads one only in a process that asked for it, which
 * marks it read while it lives (`readerOf`), so one an hour unasked and
 * unread is of a build of serde no run uses now, mostly a compiler's
 * before the last rebuild: kept a day, 62 of them took 4.1 GiB of the M5
 * Pro's main checkout (#2483). */
export const keptFor = 3600 * 1000;

/** The mark that process `pid` reads `snapshot`, beside it: there from its
 * ask to its process's end, and gone at the first sweep after. */
export const readerOf = (snapshot: string, pid = process.pid) => `${snapshot}.reader-${pid}`;

/** Of a name in a store, the mark of a reader or of a snapshot's making,
 * and whose process's: never a snapshot to keep or remove. */
const marked = (name: string) => /^(.+)\.(reader|making)-(\d+)$/.exec(name);

/** The snapshots of `store` no live process reads (`readerOf`), each with
 * when it was last asked for, and the names of those one does. Marks of
 * processes gone are removed as they're read. */
function listed(store: string): { free: { path: string; used: number }[]; read: Set<string> } {
  const names = existsSync(store) ? readdirSync(store) : [];
  const read = new Set<string>();
  for (const name of names) {
    const mark = marked(name);
    if (mark?.[2] !== "reader") continue;
    if (alive(Number(mark[3]))) read.add(mark[1]);
    else rmSync(join(store, name), { force: true });
  }
  const free = names.flatMap((name) => {
    if (marked(name) || read.has(name)) return [];
    const at = join(store, name);
    // An entry another run's sweep removed, or renamed into place, since
    // it was listed is no longer this one's to judge (crates.test.ts's
    // ENOENT, 2026-11-17).
    const seen = statSync(at, { throwIfNoEntry: false });
    return seen === undefined ? [] : [{ path: at, used: seen.mtimeMs }];
  });
  return { free, read };
}

/** The snapshots of `store` a prune may take, scripts/disk.ts' store of
 * them: those no live process reads, each with its size. */
export function serdeSnapshots(store: string): Kept[] {
  return listed(store).free.map(({ path, used }) => ({ path, used, bytes: bytesOf(path) }));
}

/** `store` swept: each snapshot no live process reads and none asked for
 * in `keptFor` removed, and the makings of a process gone. A snapshot is
 * renamed away before it's removed and its readers read again, so one a
 * run marked read meanwhile is put back, not taken from under it. How
 * many it removed. */
export function sweepSerdeKept(store: string, now = Date.now()): number {
  if (!existsSync(store)) return 0;
  for (const name of readdirSync(store)) {
    const mark = marked(name);
    if (mark?.[2] === "making" && !alive(Number(mark[3]))) rmSync(join(store, name), { recursive: true, force: true });
  }
  let removed = 0;
  for (const { path, used } of listed(store).free) {
    if (now - used <= keptFor) continue;
    const away = `${path}.making-${process.pid}`;
    try {
      renameSync(path, away);
    } catch {
      continue;
    }
    if (listed(store).read.has(basename(path))) {
      try {
        renameSync(away, path);
      } catch {
        // Its reader made it again meanwhile: its is as whole as this one.
        rmSync(away, { recursive: true, force: true });
      }
    } else {
      rmSync(away, { recursive: true, force: true });
      removed++;
    }
  }
  return removed;
}

/** `flags`, serde/build.sh's, of a snapshot in `store` of what they name:
 * each `--extern name=<file>` and `-L dependency=<dir>` its copy's, the
 * snapshot's directory named by what they hold. Made in a directory of its
 * own, then renamed into place, so two runs making one at once both find it
 * whole; marked read by this process first, so no sweep takes it while the
 * process lives, and the store swept after (`sweepSerdeKept`). */
export function keptSerdeFlags(flags: string[], store: string): string[] {
  const named = flags.map((flag) => /^(?:([\w-]+)=|dependency=)(\/.+)$/.exec(flag));
  const paths = named.flatMap((m) => (m ? [m[2]] : []));
  const hash = createHash("sha256");
  paths.forEach((path, k) => hashed(hash, path, String(k)));
  const dir = join(store, hash.digest("hex").slice(0, 24));
  mkdirSync(store, { recursive: true });
  writeFileSync(readerOf(dir), "");
  const now = new Date();
  // Made where it's not there, and again where another run's sweep had it
  // renamed away as this one marked it, till its put back or made anew.
  for (let asked = false; !asked; ) {
    if (!existsSync(dir)) {
      const making = `${dir}.making-${process.pid}`;
      rmSync(making, { recursive: true, force: true });
      paths.forEach((path, k) => copied(path, join(making, String(k), basename(path))));
      try {
        renameSync(making, dir);
      } catch {
        // Another run made it first: its is as whole as this one.
        rmSync(making, { recursive: true, force: true });
      }
    }
    try {
      utimesSync(dir, now, now);
      asked = true;
    } catch (error) {
      if (!gone(error)) throw error;
    }
  }
  sweepSerdeKept(store, now.getTime());
  let k = 0;
  return flags.map((flag, i) => {
    const m = named[i];
    if (!m) return flag;
    const kept = join(dir, String(k++), basename(m[2]));
    return m[1] ? `${m[1]}=${kept}` : `dependency=${kept}`;
  });
}
