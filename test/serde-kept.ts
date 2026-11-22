// serde's built crates as a compile reads them: a snapshot of what
// serde/build.sh's flags name, each file and directory copied, as a clone
// where the disk can, into a directory named by what they hold. A Cargo
// build of `target/serde` after, by another process, writes its own files
// anew, and never takes one from under a compile still reading it, which
// rustc says as "can't find crate for `serde`" (#1711).
import { createHash } from "node:crypto";
import { constants, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync } from "node:fs";
import { basename, dirname, join } from "node:path";

/** The files of `path` a compile reads: itself, a file's, or a directory's
 * own files, as rustc's `-L dependency=` search reads it, none of its
 * subdirectories. Under Cargo 1.100 the directory is a build step's
 * `build/<package>/<hash>/out`, where proc-macro2's build script writes and
 * removes probe files, `out/probe`, while another run's Cargo builds: a
 * walk of it found one gone between listing it and reading it (#2311).
 * Listed by each entry's type, with no stat to race. */
function read(path: string): string[] {
  if (!statSync(path).isDirectory()) return [path];
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
  mkdirSync(files.length === 1 && files[0] === from ? dirname(to) : to, { recursive: true });
  for (const file of files) {
    try {
      copy(file, file === from ? to : join(to, basename(file)));
    } catch (error) {
      if (file !== from && gone(error)) continue;
      throw error;
    }
  }
}

/** How long a snapshot no run has asked for is kept. */
const keptFor = 24 * 3600 * 1000;

/** `flags`, serde/build.sh's, of a snapshot in `store` of what they name:
 * each `--extern name=<file>` and `-L dependency=<dir>` its copy's, the
 * snapshot's directory named by what they hold. Made in a directory of its
 * own, then renamed into place, so two runs making one at once both find it
 * whole; one no run has asked for in a day is let go, and one a run left
 * half made. */
export function keptSerdeFlags(flags: string[], store: string): string[] {
  const named = flags.map((flag) => /^(?:([\w-]+)=|dependency=)(\/.+)$/.exec(flag));
  const paths = named.flatMap((m) => (m ? [m[2]] : []));
  const hash = createHash("sha256");
  paths.forEach((path, k) => hashed(hash, path, String(k)));
  const dir = join(store, hash.digest("hex").slice(0, 24));
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
  const now = new Date();
  utimesSync(dir, now, now);
  // An entry another run's sweep removed, or renamed into place, since it
  // was listed is no longer this one's to judge (crates.test.ts's ENOENT,
  // 2026-11-17).
  for (const other of readdirSync(store)) {
    const at = join(store, other);
    const seen = statSync(at, { throwIfNoEntry: false });
    if (at !== dir && seen !== undefined && now.getTime() - seen.mtimeMs > keptFor) rmSync(at, { recursive: true, force: true });
  }
  let k = 0;
  return flags.map((flag, i) => {
    const m = named[i];
    if (!m) return flag;
    const kept = join(dir, String(k++), basename(m[2]));
    return m[1] ? `${m[1]}=${kept}` : `dependency=${kept}`;
  });
}
