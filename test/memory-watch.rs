// The watchdog of a native run's memory on macOS (#1515), built and kept as
// a native program is (`nativeBinary`): `memory-watch <pid> <cap> <mark>`
// asks every 50 ms how much memory process `pid` holds, its physical
// footprint, compressed pages too, as the system's memory pressure counts
// it; past `cap` bytes, it writes what it held to `mark` and kills `pid`.
// It ends once `pid` has. macOS keeps no `RLIMIT_AS` or `RLIMIT_DATA` a
// program's allocator heeds, and `ps`'s resident size leaves out what the
// system compressed: three runs held 9 GB each that way and froze the
// machine.
use std::{env, fs, thread, time::Duration};

/// `proc_pid_rusage`'s `RUSAGE_INFO_V0` (`<sys/resource.h>`).
#[repr(C)]
struct RusageInfoV0 {
    uuid: [u8; 16],
    user_time: u64,
    system_time: u64,
    pkg_idle_wkups: u64,
    interrupt_wkups: u64,
    pageins: u64,
    wired_size: u64,
    resident_size: u64,
    phys_footprint: u64,
    proc_start_abstime: u64,
    proc_exit_abstime: u64,
}

unsafe extern "C" {
    fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut RusageInfoV0) -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
}

/// What `pid` holds, its footprint in bytes; `None` once it's gone.
fn footprint(pid: i32) -> Option<u64> {
    let mut info = std::mem::MaybeUninit::<RusageInfoV0>::zeroed();
    // SAFETY: `info` is a `RUSAGE_INFO_V0` the call writes whole, flavor 0.
    let asked = unsafe { proc_pid_rusage(pid, 0, info.as_mut_ptr()) };
    // SAFETY: zeroed, then written by the call where it succeeded.
    (asked == 0).then(|| unsafe { info.assume_init() }.phys_footprint)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let [_, pid, cap, mark] = &args[..] else {
        eprintln!("memory-watch <pid> <cap in bytes> <mark file>");
        std::process::exit(64);
    };
    let pid: i32 = pid.parse().expect("a pid");
    let cap: u64 = cap.parse().expect("a cap in bytes");
    while let Some(held) = footprint(pid) {
        if held > cap {
            let _ = fs::write(mark, held.to_string());
            // SAFETY: a signal sent to the process watched, SIGKILL.
            unsafe { kill(pid, 9) };
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}
