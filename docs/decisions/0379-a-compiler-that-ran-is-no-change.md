# 0379. A compiler that ran is no change to it

Status: Accepted. Extends [0192](0192-next.md) and
[0101](0101-cargo-workspace-wrapper.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how a host knows to
build again, not what a program means.

## Context

A dev server builds again when the compiler is replaced, a rust-js rebuilt
or upgraded while it runs, so it watches the compiler's files. On macOS,
`fs.watch` reports a file that's run as changed, as it does one replaced:
each build runs the compiler, which is the next build's event. A trial of
react.dev under `rust-js-next dev` compiled about twice a second, idle,
from its first save on, 480 times in four minutes.

Whether an event on the compiler was a change was asked twice: the Next.js
plugin's watcher took any event on a compiler file as one, through the
session's `reads`, and the Vite plugin asked `watchFiles.includes(file)`.

## Decision

**An event on the compiler is a change only where the compiler is another
since it last said what it is**: its files' sizes, times or inodes, the
key `--version-json`'s answer is kept under. The builder answers it,
`compilerChanged(file)`, and both hosts ask it, the Next.js plugin through
the session's `reads`, the Vite plugin directly; the architecture test
holds that neither asks `watchFiles.includes` itself.

`examples/next`'s committed JS is held to what rust-js writes from its
Rust, as the pilot's is: it had its `<Image>`'s props out of the order its
Rust gives them, from a compiler change no test of it saw.

## Consequences

- One save is one compile, on macOS too.
- A compiler replaced with the same size and time, on the same inode, is
  no change; a build of it, or an install, writes a new file or a new time.
