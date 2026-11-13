// serde's built crates as a compile reads them: a snapshot of what
// serde/build.sh's flags name, each file and directory copied, as a clone
// where the disk can, into a directory named by what they hold. A Cargo
// build of `target/serde` after, by another process, writes its own files
// anew, and never takes one from under a compile still reading it, which
// rustc says as "can't find crate for `serde`" (#1711).
import { createHash } from "node:crypto";
import { constants, copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync } from "node:fs";
import { basename, dirname, join } from "node:path";

/** Each file of `path`, a file or a directory's, read whole, into `hash`. */
function hashed(hash: ReturnType<typeof createHash>, path: string, as: string) {
  if (statSync(path).isDirectory()) {
    for (const entry of readdirSync(path).sort()) hashed(hash, join(path, entry), `${as}/${entry}`);
  } else {
    hash.update(`${as}\0`).update(readFileSync(path)).update("\0");
  }
}

/** `from`, a file or a directory, copied to `to`, each file a clone where
 * the disk can. */
function copied(from: string, to: string) {
  if (statSync(from).isDirectory()) {
    mkdirSync(to, { recursive: true });
    for (const entry of readdirSync(from)) copied(join(from, entry), join(to, entry));
  } else {
    mkdirSync(dirname(to), { recursive: true });
    copyFileSync(from, to, constants.COPYFILE_FICLONE);
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
  for (const other of readdirSync(store)) {
    const at = join(store, other);
    if (at !== dir && now.getTime() - statSync(at).mtimeMs > keptFor) rmSync(at, { recursive: true, force: true });
  }
  let k = 0;
  return flags.map((flag, i) => {
    const m = named[i];
    if (!m) return flag;
    const kept = join(dir, String(k++), basename(m[2]));
    return m[1] ? `${m[1]}=${kept}` : `dependency=${kept}`;
  });
}
