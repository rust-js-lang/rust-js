//! [`process`](https://nodejs.org/api/process.html): the Node process, a
//! global: its arguments, environment, platform, resources, and how it
//! ends.

use core::marker::PhantomData;
use std::collections::HashSet;

use js::{Dict, JsError, JsObject, Promise, Unknown};

use crate::event;
use crate::events::Emits;
use crate::fs::IntoPathLike;

/// [`process.abort()`](https://nodejs.org/api/process.html#processabort):
/// end the process at once, with a core file.
#[cfg_attr(rust_js, rust_js::link_name = "process.abort")]
pub fn abort() -> ! {
    unreachable!()
}

/// [`process.argv`](https://nodejs.org/api/process.html#processargv):
/// the command line, `node`'s path and the script's first.
#[cfg_attr(rust_js, rust_js::link_name = "get process.argv")]
pub fn argv() -> Vec<String> {
    unreachable!()
}

/// [`process.argv0`](https://nodejs.org/api/process.html#processargv0):
/// `argv[0]` as it was given.
#[cfg_attr(rust_js, rust_js::link_name = "get process.argv0")]
pub fn argv0() -> String {
    unreachable!()
}

/// [`process.arch`](https://nodejs.org/api/process.html#processarch):
/// the CPU's architecture Node was built for.
#[cfg_attr(rust_js, rust_js::link_name = "get process.arch")]
pub fn arch() -> Architecture {
    unreachable!()
}

/// [`process.availableMemory()`](https://nodejs.org/api/process.html#processavailablememory):
/// the bytes of memory the process may still use.
#[cfg_attr(rust_js, rust_js::link_name = "process.availableMemory")]
pub fn available_memory() -> f64 {
    unreachable!()
}

/// [`process.chdir(directory)`](https://nodejs.org/api/process.html#processchdirdirectory):
/// run in `directory`; what it throws, an `Err`.
#[cfg_attr(rust_js, rust_js::link_name = "process.chdir")]
pub fn chdir(directory: &str) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.config`](https://nodejs.org/api/process.html#processconfig):
/// how Node was built.
#[cfg_attr(rust_js, rust_js::link_name = "get process.config")]
pub fn config() -> ProcessConfig {
    unreachable!()
}

/// [`process.connected`](https://nodejs.org/api/process.html#processconnected):
/// whether its IPC channel is open, of a process spawned with one.
#[cfg_attr(rust_js, rust_js::link_name = "get process.connected")]
pub fn connected() -> bool {
    unreachable!()
}

/// [`process.constrainedMemory()`](https://nodejs.org/api/process.html#processconstrainedmemory):
/// the bytes of memory the OS lets it use, 0 where it sets no limit.
#[cfg_attr(rust_js, rust_js::link_name = "process.constrainedMemory")]
pub fn constrained_memory() -> f64 {
    unreachable!()
}

/// [`process.cpuUsage(previousValue)`](https://nodejs.org/api/process.html#processcpuusagepreviousvalue):
/// the CPU time it has used, in microseconds, or since `previous_value`.
#[cfg_attr(rust_js, rust_js::link_name = "process.cpuUsage")]
pub fn cpu_usage(previous_value: Option<CpuUsage>) -> CpuUsage {
    unreachable!()
}

/// [`process.cwd()`](https://nodejs.org/api/process.html#processcwd):
/// the directory the process runs in.
#[cfg_attr(rust_js, rust_js::link_name = "process.cwd")]
pub fn cwd() -> String {
    unreachable!()
}

/// [`process.debugPort`](https://nodejs.org/api/process.html#processdebugport):
/// the inspector's port.
#[cfg_attr(rust_js, rust_js::link_name = "get process.debugPort")]
pub fn debug_port() -> f64 {
    unreachable!()
}

/// [`process.disconnect()`](https://nodejs.org/api/process.html#processdisconnect):
/// close its IPC channel to its parent.
#[cfg_attr(rust_js, rust_js::link_name = "process.disconnect")]
pub fn disconnect() {
    unreachable!()
}

/// [`process.env`](https://nodejs.org/api/process.html#processenv): its
/// environment, each variable's value, read and written.
#[cfg_attr(rust_js, rust_js::link_name = "get process.env")]
pub fn env() -> &'static ProcessEnv {
    unreachable!()
}

/// [`process.execArgv`](https://nodejs.org/api/process.html#processexecargv):
/// Node's own options, before the script.
#[cfg_attr(rust_js, rust_js::link_name = "get process.execArgv")]
pub fn exec_argv() -> Vec<String> {
    unreachable!()
}

/// [`process.execPath`](https://nodejs.org/api/process.html#processexecpath):
/// the path of `node` itself.
#[cfg_attr(rust_js, rust_js::link_name = "get process.execPath")]
pub fn exec_path() -> String {
    unreachable!()
}

/// [`process.execve(file, args, env)`](https://nodejs.org/api/process.html#processexecvefile-args-env):
/// replace the process with `file`'s, of `args` and `env`, or none and
/// [`env`]; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.execve")]
pub fn execve(file: &str, args: Option<&[&str]>, env: Option<&ProcessEnv>) -> ! {
    unreachable!()
}

/// [`process.exit(code)`](https://nodejs.org/api/process.html#processexitcode):
/// end the process, of `code`, or [`exit_code`]'s, or 0.
#[cfg_attr(rust_js, rust_js::link_name = "process.exit")]
pub fn exit(code: Option<&dyn IntoNumberOrStr>) -> ! {
    unreachable!()
}

/// [`process.features`](https://nodejs.org/api/process.html#processfeatures):
/// what this Node can do.
#[cfg_attr(rust_js, rust_js::link_name = "get process.features")]
pub fn features() -> ProcessFeatures {
    unreachable!()
}

/// [`process.finalization`](https://nodejs.org/api/process.html#processfinalizationregisterref-callback):
/// callbacks run as the process ends, while their objects live.
#[cfg_attr(rust_js, rust_js::link_name = "get process.finalization")]
pub fn finalization() -> &'static ProcessFinalization {
    unreachable!()
}

/// [`process.getActiveResourcesInfo()`](https://nodejs.org/api/process.html#processgetactiveresourcesinfo):
/// the kinds of what keeps the event loop running, `"TTYWrap"`.
#[cfg_attr(rust_js, rust_js::link_name = "process.getActiveResourcesInfo")]
pub fn get_active_resources_info() -> Vec<String> {
    unreachable!()
}

/// [`process.getBuiltinModule(id)`](https://nodejs.org/api/process.html#processgetbuiltinmoduleid):
/// Node's module `id`, `"fs"`, loaded now, or `None` where there's none.
#[cfg_attr(rust_js, rust_js::link_name = "process.getBuiltinModule")]
pub fn get_builtin_module(id: &str) -> Option<&'static JsObject> {
    unreachable!()
}

/// [`process.getegid()`](https://nodejs.org/api/process.html#processgetegid):
/// its effective group's id; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.getegid")]
pub fn getegid() -> f64 {
    unreachable!()
}

/// [`process.geteuid()`](https://nodejs.org/api/process.html#processgeteuid):
/// its effective user's id; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.geteuid")]
pub fn geteuid() -> f64 {
    unreachable!()
}

/// [`process.getgid()`](https://nodejs.org/api/process.html#processgetgid):
/// its group's id; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.getgid")]
pub fn getgid() -> f64 {
    unreachable!()
}

/// [`process.getgroups()`](https://nodejs.org/api/process.html#processgetgroups):
/// its supplementary groups' ids; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.getgroups")]
pub fn getgroups() -> Vec<f64> {
    unreachable!()
}

/// [`process.getuid()`](https://nodejs.org/api/process.html#processgetuid):
/// its user's id; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.getuid")]
pub fn getuid() -> f64 {
    unreachable!()
}

/// [`process.hasUncaughtExceptionCaptureCallback()`](https://nodejs.org/api/process.html#processhasuncaughtexceptioncapturecallback):
/// whether [`set_uncaught_exception_capture_callback`] set one.
#[cfg_attr(rust_js, rust_js::link_name = "process.hasUncaughtExceptionCaptureCallback")]
pub fn has_uncaught_exception_capture_callback() -> bool {
    unreachable!()
}

/// [`process.hrtime(time)`](https://nodejs.org/api/process.html#processhrtimetime):
/// the time, `(seconds, nanoseconds)`, from a moment in the past, or since
/// `time`. Legacy: [`hrtime_bigint`] is one number.
#[cfg_attr(rust_js, rust_js::link_name = "process.hrtime")]
pub fn hrtime(time: Option<(f64, f64)>) -> (f64, f64) {
    unreachable!()
}

/// [`process.hrtime.bigint()`](https://nodejs.org/api/process.html#processhrtimebigint):
/// the time in nanoseconds from a moment in the past.
#[cfg_attr(rust_js, rust_js::link_name = "process.hrtime.bigint")]
pub fn hrtime_bigint() -> u64 {
    unreachable!()
}

/// [`process.loadEnvFile(path)`](https://nodejs.org/api/process.html#processloadenvfilepath):
/// read `path`, or `./.env`, into [`env`]; what it throws, an `Err`.
#[cfg_attr(rust_js, rust_js::link_name = "process.loadEnvFile")]
pub fn load_env_file(path: Option<&dyn IntoPathLike>) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.memoryUsage()`](https://nodejs.org/api/process.html#processmemoryusage):
/// the bytes of memory it uses.
#[cfg_attr(rust_js, rust_js::link_name = "process.memoryUsage")]
pub fn memory_usage() -> MemoryUsage {
    unreachable!()
}

/// [`process.memoryUsage.rss()`](https://nodejs.org/api/process.html#processmemoryusagerss):
/// its resident set size, quicker than [`memory_usage`].
#[cfg_attr(rust_js, rust_js::link_name = "process.memoryUsage.rss")]
pub fn memory_usage_rss() -> f64 {
    unreachable!()
}

/// [`process.nextTick(callback)`](https://nodejs.org/api/process.html#processnexttickcallback-args):
/// run `callback` once this operation is done, before the event loop goes on.
#[cfg_attr(rust_js, rust_js::link_name = "process.nextTick")]
pub fn next_tick(callback: Box<dyn FnOnce()>) {
    unreachable!()
}

/// [`process.noDeprecation`](https://nodejs.org/api/process.html#processnodeprecation):
/// whether deprecation warnings are silenced.
#[cfg_attr(rust_js, rust_js::link_name = "get process.noDeprecation")]
pub fn no_deprecation() -> Option<bool> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.noDeprecation")]
pub fn set_no_deprecation(value: bool) {
    unreachable!()
}

/// [`process.permission`](https://nodejs.org/api/process.html#processpermission):
/// what the permission model lets it do.
#[cfg_attr(rust_js, rust_js::link_name = "get process.permission")]
pub fn permission() -> &'static ProcessPermission {
    unreachable!()
}

/// [`process.pid`](https://nodejs.org/api/process.html#processpid).
#[cfg_attr(rust_js, rust_js::link_name = "get process.pid")]
pub fn pid() -> f64 {
    unreachable!()
}

/// [`process.platform`](https://nodejs.org/api/process.html#processplatform):
/// the OS Node was built for.
#[cfg_attr(rust_js, rust_js::link_name = "get process.platform")]
pub fn platform() -> Platform {
    unreachable!()
}

/// [`process.ppid`](https://nodejs.org/api/process.html#processppid):
/// its parent's pid.
#[cfg_attr(rust_js, rust_js::link_name = "get process.ppid")]
pub fn ppid() -> f64 {
    unreachable!()
}

/// [`process.release`](https://nodejs.org/api/process.html#processrelease):
/// this Node's release.
#[cfg_attr(rust_js, rust_js::link_name = "get process.release")]
pub fn release() -> ProcessRelease {
    unreachable!()
}

/// [`process.report`](https://nodejs.org/api/process.html#processreport):
/// its diagnostic reports.
#[cfg_attr(rust_js, rust_js::link_name = "get process.report")]
pub fn report() -> &'static ProcessReport {
    unreachable!()
}

/// [`process.resourceUsage()`](https://nodejs.org/api/process.html#processresourceusage):
/// what it has used of the OS's resources.
#[cfg_attr(rust_js, rust_js::link_name = "process.resourceUsage")]
pub fn resource_usage() -> ResourceUsage {
    unreachable!()
}

/// [`process.setgroups(groups)`](https://nodejs.org/api/process.html#processsetgroupsgroups):
/// set its supplementary groups, by id or name; POSIX, and root, only.
#[cfg_attr(rust_js, rust_js::link_name = "process.setgroups")]
pub fn setgroups(groups: &[NumberOrStr<'_>]) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.setSourceMapsEnabled(value)`](https://nodejs.org/api/process.html#processsetsourcemapsenabledval):
/// whether its stack traces follow source maps.
#[cfg_attr(rust_js, rust_js::link_name = "process.setSourceMapsEnabled")]
pub fn set_source_maps_enabled(value: bool) {
    unreachable!()
}

/// [`process.setUncaughtExceptionCaptureCallback(fn)`](https://nodejs.org/api/process.html#processsetuncaughtexceptioncapturecallbackfn):
/// what's called with an uncaught exception, in place of ending the
/// process; `None`, nothing.
#[cfg_attr(rust_js, rust_js::link_name = "process.setUncaughtExceptionCaptureCallback")]
#[cfg_attr(rust_js, rust_js::nullable(cb))]
pub fn set_uncaught_exception_capture_callback(cb: Option<Box<dyn Fn(&JsError)>>) {
    unreachable!()
}

/// [`process.sourceMapsEnabled`](https://nodejs.org/api/process.html#processsourcemapsenabled).
#[cfg_attr(rust_js, rust_js::link_name = "get process.sourceMapsEnabled")]
pub fn source_maps_enabled() -> bool {
    unreachable!()
}

/// [`process.threadCpuUsage(previousValue)`](https://nodejs.org/api/process.html#processthreadcpuusagepreviousvalue):
/// the CPU time this thread has used, in microseconds, or since
/// `previous_value`.
#[cfg_attr(rust_js, rust_js::link_name = "process.threadCpuUsage")]
pub fn thread_cpu_usage(previous_value: Option<CpuUsage>) -> CpuUsage {
    unreachable!()
}

/// [`process.throwDeprecation`](https://nodejs.org/api/process.html#processthrowdeprecation):
/// whether a deprecation warning is thrown.
#[cfg_attr(rust_js, rust_js::link_name = "get process.throwDeprecation")]
pub fn throw_deprecation() -> bool {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.throwDeprecation")]
pub fn set_throw_deprecation(value: bool) {
    unreachable!()
}

/// [`process.title`](https://nodejs.org/api/process.html#processtitle):
/// its name, as `ps` shows it.
#[cfg_attr(rust_js, rust_js::link_name = "get process.title")]
pub fn title() -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.title")]
pub fn set_title(value: &str) {
    unreachable!()
}

/// [`process.traceDeprecation`](https://nodejs.org/api/process.html#processtracedeprecation):
/// whether a deprecation warning prints its stack.
#[cfg_attr(rust_js, rust_js::link_name = "get process.traceDeprecation")]
pub fn trace_deprecation() -> bool {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.traceDeprecation")]
pub fn set_trace_deprecation(value: bool) {
    unreachable!()
}

/// [`process.traceProcessWarnings`](https://nodejs.org/api/process.html#processtraceprocesswarnings):
/// whether a warning prints its stack.
#[cfg_attr(rust_js, rust_js::link_name = "get process.traceProcessWarnings")]
pub fn trace_process_warnings() -> bool {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.traceProcessWarnings")]
pub fn set_trace_process_warnings(value: bool) {
    unreachable!()
}

/// [`process.umask()`](https://nodejs.org/api/process.html#processumask):
/// its file mode mask, `mask` where it's given, which it gives the old
/// one of. Reading it, without one, is deprecated: it races.
#[cfg_attr(rust_js, rust_js::link_name = "process.umask")]
pub fn umask(mask: Option<&dyn IntoNumberOrStr>) -> f64 {
    unreachable!()
}

/// [`process.uptime()`](https://nodejs.org/api/process.html#processuptime):
/// the seconds it has run.
#[cfg_attr(rust_js, rust_js::link_name = "process.uptime")]
pub fn uptime() -> f64 {
    unreachable!()
}

/// [`process.version`](https://nodejs.org/api/process.html#processversion):
/// Node's, `"v24.17.0"`.
#[cfg_attr(rust_js, rust_js::link_name = "get process.version")]
pub fn version() -> String {
    unreachable!()
}

/// [`process.versions`](https://nodejs.org/api/process.html#processversions):
/// Node's and each of its dependencies', `node` and `v8` among them.
#[cfg_attr(rust_js, rust_js::link_name = "get process.versions")]
pub fn versions() -> &'static ProcessVersions {
    unreachable!()
}

/// [`process.allowedNodeEnvironmentFlags`](https://nodejs.org/api/process.html#processallowednodeenvironmentflags):
/// the flags `NODE_OPTIONS` may hold. Adding to it does nothing.
#[cfg_attr(rust_js, rust_js::link_name = "get process.allowedNodeEnvironmentFlags")]
pub fn allowed_node_environment_flags() -> HashSet<String> {
    unreachable!()
}

/// [`process.dlopen(module, filename, flags)`](https://nodejs.org/api/process.html#processdlopenmodule-filename-flags):
/// load the shared object `filename` into `module`'s `exports`, of
/// `os.constants.dlopen`'s `flags`, or `RTLD_LAZY`.
#[cfg_attr(rust_js, rust_js::link_name = "process.dlopen")]
pub fn dlopen<M>(module: &M, filename: &str, flags: Option<f64>) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.emitWarning(warning, type, code)`](https://nodejs.org/api/process.html#processemitwarningwarning-type-code-ctor):
/// warn, a text or an `Error`, as a `"warning"` event and on stderr, a
/// warning of `type`'s name and of `code`, where they're given.
#[cfg_attr(rust_js, rust_js::link_name = "process.emitWarning")]
pub fn emit_warning(warning: impl IntoStrOrError, r#type: Option<&str>, code: Option<&str>) {
    unreachable!()
}

/// [`process.emitWarning(warning, options)`](https://nodejs.org/api/process.html#processemitwarningwarning-options):
/// a warning of `options`' type, code and detail.
#[cfg_attr(rust_js, rust_js::link_name = "process.emitWarning")]
pub fn emit_warning_with_options(warning: impl IntoStrOrError, options: EmitWarningOptions<'_>) {
    unreachable!()
}

/// [`process.exitCode`](https://nodejs.org/api/process.html#processexitcode_1):
/// the code it ends with, where it isn't given one.
#[cfg_attr(rust_js, rust_js::link_name = "get process.exitCode")]
pub fn exit_code() -> Option<NumberOrStr<'static>> {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set process.exitCode")]
pub fn set_exit_code(value: impl IntoNumberOrStr) {
    unreachable!()
}

/// [`process.kill(pid, signal)`](https://nodejs.org/api/process.html#processkillpid-signal):
/// send `pid` `signal`, `"SIGINT"` or its number, or `SIGTERM`; what it
/// throws, an `Err`.
#[cfg_attr(rust_js, rust_js::link_name = "process.kill")]
pub fn kill(pid: f64, signal: Option<&dyn IntoNumberOrStr>) -> Result<bool, &'static JsError> {
    unreachable!()
}

/// [`process.ref(maybeRefable)`](https://nodejs.org/api/process.html#processrefmayberefable):
/// keep the event loop running for `maybe_refable`, a timer say.
#[cfg_attr(rust_js, rust_js::link_name = "process.ref")]
pub fn r#ref<T>(maybe_refable: &T) {
    unreachable!()
}

/// [`process.unref(maybeRefable)`](https://nodejs.org/api/process.html#processunrefmayberefable):
/// let the event loop end, though `maybe_refable` waits.
#[cfg_attr(rust_js, rust_js::link_name = "process.unref")]
pub fn unref<T>(maybe_refable: &T) {
    unreachable!()
}

/// [`process.setegid(id)`](https://nodejs.org/api/process.html#processsetegidid):
/// set its effective group, by id or name; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.setegid")]
pub fn setegid(id: impl IntoNumberOrStr) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.seteuid(id)`](https://nodejs.org/api/process.html#processseteuidid):
/// set its effective user, by id or name; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.seteuid")]
pub fn seteuid(id: impl IntoNumberOrStr) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.setgid(id)`](https://nodejs.org/api/process.html#processsetgidid):
/// set its group, by id or name; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.setgid")]
pub fn setgid(id: impl IntoNumberOrStr) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.setuid(id)`](https://nodejs.org/api/process.html#processsetuidid):
/// set its user, by id or name; POSIX only.
#[cfg_attr(rust_js, rust_js::link_name = "process.setuid")]
pub fn setuid(id: impl IntoNumberOrStr) -> Result<(), &'static JsError> {
    unreachable!()
}

/// [`process.env`](env): each variable's value.
pub type ProcessEnv = Dict<String>;

/// [`process.versions`](versions): each version by its name.
pub type ProcessVersions = Dict<String>;

/// What [`cpu_usage`] gives, and takes to count from: microseconds.
pub struct CpuUsage {
    pub user: f64,
    pub system: f64,
}

/// What [`memory_usage`] gives: bytes.
pub struct MemoryUsage {
    pub rss: f64,
    #[cfg_attr(rust_js, rust_js::name = "heapTotal")]
    pub heap_total: f64,
    #[cfg_attr(rust_js, rust_js::name = "heapUsed")]
    pub heap_used: f64,
    pub external: f64,
    #[cfg_attr(rust_js, rust_js::name = "arrayBuffers")]
    pub array_buffers: f64,
}

/// What [`resource_usage`] gives, as `getrusage(2)` has it.
pub struct ResourceUsage {
    #[cfg_attr(rust_js, rust_js::name = "fsRead")]
    pub fs_read: f64,
    #[cfg_attr(rust_js, rust_js::name = "fsWrite")]
    pub fs_write: f64,
    #[cfg_attr(rust_js, rust_js::name = "involuntaryContextSwitches")]
    pub involuntary_context_switches: f64,
    #[cfg_attr(rust_js, rust_js::name = "ipcReceived")]
    pub ipc_received: f64,
    #[cfg_attr(rust_js, rust_js::name = "ipcSent")]
    pub ipc_sent: f64,
    #[cfg_attr(rust_js, rust_js::name = "majorPageFault")]
    pub major_page_fault: f64,
    #[cfg_attr(rust_js, rust_js::name = "maxRSS")]
    pub max_rss: f64,
    #[cfg_attr(rust_js, rust_js::name = "minorPageFault")]
    pub minor_page_fault: f64,
    #[cfg_attr(rust_js, rust_js::name = "sharedMemorySize")]
    pub shared_memory_size: f64,
    #[cfg_attr(rust_js, rust_js::name = "signalsCount")]
    pub signals_count: f64,
    #[cfg_attr(rust_js, rust_js::name = "swappedOut")]
    pub swapped_out: f64,
    #[cfg_attr(rust_js, rust_js::name = "systemCPUTime")]
    pub system_cpu_time: f64,
    #[cfg_attr(rust_js, rust_js::name = "unsharedDataSize")]
    pub unshared_data_size: f64,
    #[cfg_attr(rust_js, rust_js::name = "unsharedStackSize")]
    pub unshared_stack_size: f64,
    #[cfg_attr(rust_js, rust_js::name = "userCPUTime")]
    pub user_cpu_time: f64,
    #[cfg_attr(rust_js, rust_js::name = "voluntaryContextSwitches")]
    pub voluntary_context_switches: f64,
}

/// What [`release`] gives: this Node's release, and where its source and
/// headers are.
pub struct ProcessRelease {
    /// `"node"`.
    pub name: String,
    #[cfg_attr(rust_js, rust_js::name = "sourceUrl")]
    pub source_url: Option<String>,
    #[cfg_attr(rust_js, rust_js::name = "headersUrl")]
    pub headers_url: Option<String>,
    #[cfg_attr(rust_js, rust_js::name = "libUrl")]
    pub lib_url: Option<String>,
    /// Its LTS line's name, `"Krypton"`, of an LTS release.
    pub lts: Option<String>,
}

/// What [`features`] gives: what this Node can do.
pub struct ProcessFeatures {
    #[cfg_attr(rust_js, rust_js::name = "cached_builtins")]
    pub cached_builtins: bool,
    pub debug: bool,
    pub inspector: bool,
    pub ipv6: bool,
    #[cfg_attr(rust_js, rust_js::name = "require_module")]
    pub require_module: bool,
    pub tls: bool,
    #[cfg_attr(rust_js, rust_js::name = "tls_alpn")]
    pub tls_alpn: bool,
    #[cfg_attr(rust_js, rust_js::name = "tls_ocsp")]
    pub tls_ocsp: bool,
    #[cfg_attr(rust_js, rust_js::name = "tls_sni")]
    pub tls_sni: bool,
    /// How it runs TypeScript: by stripping its types, by transforming it, or not.
    pub typescript: ProcessFeaturesTypeScript,
    pub uv: bool,
}

/// [`ProcessFeatures::typescript`]: `"strip"`, `"transform"`, or `false`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ProcessFeaturesTypeScript {
    #[cfg_attr(rust_js, rust_js::name = "strip")]
    Strip,
    #[cfg_attr(rust_js, rust_js::name = "transform")]
    Transform,
    /// `false`: it doesn't.
    None(bool),
}

/// What [`config`] gives: how Node was built.
pub struct ProcessConfig {
    #[cfg_attr(rust_js, rust_js::name = "target_defaults")]
    pub target_defaults: ProcessConfigTargetDefaults,
    pub variables: ProcessConfigVariables,
}

/// [`ProcessConfig::target_defaults`].
pub struct ProcessConfigTargetDefaults {
    pub cflags: Vec<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::name = "default_configuration")]
    pub default_configuration: String,
    pub defines: Vec<String>,
    #[cfg_attr(rust_js, rust_js::name = "include_dirs")]
    pub include_dirs: Vec<String>,
    pub libraries: Vec<String>,
}

/// [`ProcessConfig::variables`].
pub struct ProcessConfigVariables {
    pub clang: f64,
    #[cfg_attr(rust_js, rust_js::name = "host_arch")]
    pub host_arch: String,
    #[cfg_attr(rust_js, rust_js::name = "node_install_npm")]
    pub node_install_npm: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_install_waf")]
    pub node_install_waf: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_prefix")]
    pub node_prefix: String,
    #[cfg_attr(rust_js, rust_js::name = "node_shared_openssl")]
    pub node_shared_openssl: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_shared_v8")]
    pub node_shared_v8: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_shared_zlib")]
    pub node_shared_zlib: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_use_dtrace")]
    pub node_use_dtrace: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_use_etw")]
    pub node_use_etw: bool,
    #[cfg_attr(rust_js, rust_js::name = "node_use_openssl")]
    pub node_use_openssl: bool,
    #[cfg_attr(rust_js, rust_js::name = "target_arch")]
    pub target_arch: String,
    #[cfg_attr(rust_js, rust_js::name = "v8_no_strict_aliasing")]
    pub v8_no_strict_aliasing: f64,
    #[cfg_attr(rust_js, rust_js::name = "v8_use_snapshot")]
    pub v8_use_snapshot: bool,
    pub visibility: String,
}

/// What [`emit_warning_with_options`] takes.
#[derive(Default)]
pub struct EmitWarningOptions<'a> {
    /// Its name, `"Warning"` where `None`.
    pub r#type: Option<&'a str>,
    /// Its code, `"DEP0001"`.
    pub code: Option<&'a str>,
    /// More of what it says.
    pub detail: Option<&'a str>,
}

/// [`finalization`]'s: callbacks run as the process ends.
pub struct ProcessFinalization(PhantomData<JsObject>);

impl ProcessFinalization {
    /// Call `callback` with `r#ref` as the process exits, if `r#ref` lives.
    #[cfg_attr(rust_js, rust_js::link_name = "register")]
    pub fn register<T: 'static>(&self, r#ref: &T, callback: impl Fn(&T, &str) + 'static) {
        unreachable!()
    }

    /// Call `callback` with `r#ref` before the process exits, if `r#ref` lives.
    #[cfg_attr(rust_js, rust_js::link_name = "registerBeforeExit")]
    pub fn register_before_exit<T: 'static>(&self, r#ref: &T, callback: impl Fn(&T, &str) + 'static) {
        unreachable!()
    }

    /// Call no callback of `r#ref`.
    #[cfg_attr(rust_js, rust_js::link_name = "unregister")]
    pub fn unregister<T>(&self, r#ref: &T) {
        unreachable!()
    }
}

/// [`permission`]'s: what the permission model lets the process do.
pub struct ProcessPermission(PhantomData<JsObject>);

impl ProcessPermission {
    /// Whether it may use `scope`, `"fs.read"`, of `reference`, a path say,
    /// where it's given.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, scope: &str, reference: Option<&str>) -> bool {
        unreachable!()
    }
}

/// [`report`]'s: diagnostic reports, and when they're written.
pub struct ProcessReport(PhantomData<JsObject>);

impl ProcessReport {
    #[cfg_attr(rust_js, rust_js::link_name = "get compact")]
    pub fn compact(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set compact")]
    pub fn set_compact(&self, value: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get directory")]
    pub fn directory(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set directory")]
    pub fn set_directory(&self, value: &str) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get filename")]
    pub fn filename(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set filename")]
    pub fn set_filename(&self, value: &str) {
        unreachable!()
    }

    /// A report, as an object, of now, or of `err`'s stack.
    #[cfg_attr(rust_js, rust_js::link_name = "getReport")]
    pub fn get_report(&self, err: Option<&JsError>) -> &'static JsObject {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get reportOnFatalError")]
    pub fn report_on_fatal_error(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set reportOnFatalError")]
    pub fn set_report_on_fatal_error(&self, value: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get reportOnSignal")]
    pub fn report_on_signal(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set reportOnSignal")]
    pub fn set_report_on_signal(&self, value: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get reportOnUncaughtException")]
    pub fn report_on_uncaught_exception(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set reportOnUncaughtException")]
    pub fn set_report_on_uncaught_exception(&self, value: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get excludeEnv")]
    pub fn exclude_env(&self) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set excludeEnv")]
    pub fn set_exclude_env(&self, value: bool) {
        unreachable!()
    }

    /// The signal that writes a report.
    #[cfg_attr(rust_js, rust_js::link_name = "get signal")]
    pub fn signal(&self) -> Signals {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set signal")]
    pub fn set_signal(&self, value: Signals) {
        unreachable!()
    }

    /// Write a report, to `file_name`, of `err`'s stack, where they're
    /// given, and give its file's name.
    #[cfg_attr(rust_js, rust_js::link_name = "writeReport")]
    pub fn write_report(&self, file_name: Option<&str>, err: Option<&JsError>) -> String {
        unreachable!()
    }
}

/// An OS [`platform`] Node is built for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    #[cfg_attr(rust_js, rust_js::name = "aix")]
    Aix,
    #[cfg_attr(rust_js, rust_js::name = "android")]
    Android,
    #[cfg_attr(rust_js, rust_js::name = "darwin")]
    Darwin,
    #[cfg_attr(rust_js, rust_js::name = "freebsd")]
    Freebsd,
    #[cfg_attr(rust_js, rust_js::name = "haiku")]
    Haiku,
    #[cfg_attr(rust_js, rust_js::name = "linux")]
    Linux,
    #[cfg_attr(rust_js, rust_js::name = "openbsd")]
    Openbsd,
    #[cfg_attr(rust_js, rust_js::name = "sunos")]
    Sunos,
    #[cfg_attr(rust_js, rust_js::name = "win32")]
    Win32,
    #[cfg_attr(rust_js, rust_js::name = "cygwin")]
    Cygwin,
    #[cfg_attr(rust_js, rust_js::name = "netbsd")]
    Netbsd,
}

/// A CPU [`arch`]itecture Node is built for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Architecture {
    #[cfg_attr(rust_js, rust_js::name = "arm")]
    Arm,
    #[cfg_attr(rust_js, rust_js::name = "arm64")]
    Arm64,
    #[cfg_attr(rust_js, rust_js::name = "ia32")]
    Ia32,
    #[cfg_attr(rust_js, rust_js::name = "loong64")]
    Loong64,
    #[cfg_attr(rust_js, rust_js::name = "mips")]
    Mips,
    #[cfg_attr(rust_js, rust_js::name = "mipsel")]
    Mipsel,
    #[cfg_attr(rust_js, rust_js::name = "ppc64")]
    Ppc64,
    #[cfg_attr(rust_js, rust_js::name = "riscv64")]
    Riscv64,
    #[cfg_attr(rust_js, rust_js::name = "s390x")]
    S390x,
    #[cfg_attr(rust_js, rust_js::name = "x64")]
    X64,
}

/// A POSIX signal, by its name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signals {
    #[cfg_attr(rust_js, rust_js::name = "SIGABRT")]
    Sigabrt,
    #[cfg_attr(rust_js, rust_js::name = "SIGALRM")]
    Sigalrm,
    #[cfg_attr(rust_js, rust_js::name = "SIGBUS")]
    Sigbus,
    #[cfg_attr(rust_js, rust_js::name = "SIGCHLD")]
    Sigchld,
    #[cfg_attr(rust_js, rust_js::name = "SIGCONT")]
    Sigcont,
    #[cfg_attr(rust_js, rust_js::name = "SIGFPE")]
    Sigfpe,
    #[cfg_attr(rust_js, rust_js::name = "SIGHUP")]
    Sighup,
    #[cfg_attr(rust_js, rust_js::name = "SIGILL")]
    Sigill,
    #[cfg_attr(rust_js, rust_js::name = "SIGINT")]
    Sigint,
    #[cfg_attr(rust_js, rust_js::name = "SIGIO")]
    Sigio,
    #[cfg_attr(rust_js, rust_js::name = "SIGIOT")]
    Sigiot,
    #[cfg_attr(rust_js, rust_js::name = "SIGKILL")]
    Sigkill,
    #[cfg_attr(rust_js, rust_js::name = "SIGPIPE")]
    Sigpipe,
    #[cfg_attr(rust_js, rust_js::name = "SIGPOLL")]
    Sigpoll,
    #[cfg_attr(rust_js, rust_js::name = "SIGPROF")]
    Sigprof,
    #[cfg_attr(rust_js, rust_js::name = "SIGPWR")]
    Sigpwr,
    #[cfg_attr(rust_js, rust_js::name = "SIGQUIT")]
    Sigquit,
    #[cfg_attr(rust_js, rust_js::name = "SIGSEGV")]
    Sigsegv,
    #[cfg_attr(rust_js, rust_js::name = "SIGSTKFLT")]
    Sigstkflt,
    #[cfg_attr(rust_js, rust_js::name = "SIGSTOP")]
    Sigstop,
    #[cfg_attr(rust_js, rust_js::name = "SIGSYS")]
    Sigsys,
    #[cfg_attr(rust_js, rust_js::name = "SIGTERM")]
    Sigterm,
    #[cfg_attr(rust_js, rust_js::name = "SIGTRAP")]
    Sigtrap,
    #[cfg_attr(rust_js, rust_js::name = "SIGTSTP")]
    Sigtstp,
    #[cfg_attr(rust_js, rust_js::name = "SIGTTIN")]
    Sigttin,
    #[cfg_attr(rust_js, rust_js::name = "SIGTTOU")]
    Sigttou,
    #[cfg_attr(rust_js, rust_js::name = "SIGUNUSED")]
    Sigunused,
    #[cfg_attr(rust_js, rust_js::name = "SIGURG")]
    Sigurg,
    #[cfg_attr(rust_js, rust_js::name = "SIGUSR1")]
    Sigusr1,
    #[cfg_attr(rust_js, rust_js::name = "SIGUSR2")]
    Sigusr2,
    #[cfg_attr(rust_js, rust_js::name = "SIGVTALRM")]
    Sigvtalrm,
    #[cfg_attr(rust_js, rust_js::name = "SIGWINCH")]
    Sigwinch,
    #[cfg_attr(rust_js, rust_js::name = "SIGXCPU")]
    Sigxcpu,
    #[cfg_attr(rust_js, rust_js::name = "SIGXFSZ")]
    Sigxfsz,
    #[cfg_attr(rust_js, rust_js::name = "SIGBREAK")]
    Sigbreak,
    #[cfg_attr(rust_js, rust_js::name = "SIGLOST")]
    Siglost,
    #[cfg_attr(rust_js, rust_js::name = "SIGINFO")]
    Siginfo,
}

/// A number or a text, `exitCode`'s, as read.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum NumberOrStr<'a> {
    Number(f64),
    Str(&'a str),
}

/// What a `number | string` parameter takes: each as it is (ADR 0229); an
/// optional one, `Option<&dyn IntoNumberOrStr>`, borrowed, `Some(&value)`
/// (ADR 0443).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `number | string`",
    note = "an optional one, `Option<&dyn IntoNumberOrStr>`, takes `Some(&value)`"
)]
#[cfg_attr(rust_js, rust_js::types = "number | string")]
pub trait IntoNumberOrStr: sealed::Sealed {}
impl IntoNumberOrStr for f64 {}
impl IntoNumberOrStr for &str {}
impl IntoNumberOrStr for NumberOrStr<'_> {}

/// What a `string | Error` parameter takes: each as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | Error`")]
#[cfg_attr(rust_js, rust_js::types = "string | Error")]
pub trait IntoStrOrError: sealed::Sealed {}
impl IntoStrOrError for &str {}
impl IntoStrOrError for &JsError {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f64 {}
    impl Sealed for &str {}
    impl Sealed for super::NumberOrStr<'_> {}
    impl Sealed for &js::JsError {}
}

/// The `process` global's type, of its events (ADR 0361): `process::on`'s.
#[cfg_attr(rust_js, rust_js::types = "NodeJS.Process")]
pub struct Process(PhantomData<JsObject>);

/// Where an uncaught exception came from: a throw, or a rejected promise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UncaughtExceptionOrigin {
    #[cfg_attr(rust_js, rust_js::name = "uncaughtException")]
    UncaughtException,
    #[cfg_attr(rust_js, rust_js::name = "unhandledRejection")]
    UnhandledRejection,
}

/// A rejected promise, of whatever it was to give.
pub type AnyPromise = Promise<Option<&'static Unknown>>;

impl Emits<event::BeforeExit> for Process {
    type Listener = dyn Fn(f64);
    type Args = (f64,);
}

impl Emits<event::Disconnect> for Process {
    type Listener = dyn Fn();
    type Args = ();
}

impl Emits<event::Exit> for Process {
    type Listener = dyn Fn(f64);
    type Args = (f64,);
}

impl Emits<event::RejectionHandled> for Process {
    type Listener = dyn Fn(&AnyPromise);
    type Args = (&'static AnyPromise,);
}

impl Emits<event::UncaughtException> for Process {
    type Listener = dyn Fn(&JsError, UncaughtExceptionOrigin);
    type Args = (&'static JsError, UncaughtExceptionOrigin);
}

impl Emits<event::UncaughtExceptionMonitor> for Process {
    type Listener = dyn Fn(&JsError, UncaughtExceptionOrigin);
    type Args = (&'static JsError, UncaughtExceptionOrigin);
}

impl Emits<event::UnhandledRejection> for Process {
    type Listener = dyn Fn(Option<&Unknown>, &AnyPromise);
    type Args = (Option<&'static Unknown>, &'static AnyPromise);
}

impl Emits<event::Warning> for Process {
    type Listener = dyn Fn(&JsError);
    type Args = (&'static JsError,);
}

impl Emits<event::WorkerMessage> for Process {
    type Listener = dyn Fn(Option<&Unknown>, f64);
    type Args = (Option<&'static Unknown>, f64);
}

/// A signal, its name the event's: `process::on(Signals::Sigint, ..)`.
impl Emits<Signals> for Process {
    type Listener = dyn Fn(Signals);
    type Args = (Signals,);
}

/// [`process.on(event, listener)`](https://nodejs.org/api/process.html#process-events):
/// call `listener` at each of the process's `event`.
#[cfg_attr(rust_js, rust_js::link_name = "process.on")]
pub fn on<E>(event: E, listener: Box<<Process as Emits<E>>::Listener>)
where
    Process: Emits<E>,
{
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "process.addListener")]
pub fn add_listener<E>(event: E, listener: Box<<Process as Emits<E>>::Listener>)
where
    Process: Emits<E>,
{
    unreachable!()
}

/// `process.once(event, listener)`: at its next `event` only.
#[cfg_attr(rust_js, rust_js::link_name = "process.once")]
pub fn once<E>(event: E, listener: Box<<Process as Emits<E>>::Listener>)
where
    Process: Emits<E>,
{
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "process.prependListener")]
pub fn prepend_listener<E>(event: E, listener: Box<<Process as Emits<E>>::Listener>)
where
    Process: Emits<E>,
{
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "process.prependOnceListener")]
pub fn prepend_once_listener<E>(event: E, listener: Box<<Process as Emits<E>>::Listener>)
where
    Process: Emits<E>,
{
    unreachable!()
}

/// `process.off(event, listener)`: call `listener` no more, one
/// [`events::listener`](crate::events::listener) made.
#[cfg_attr(rust_js, rust_js::link_name = "process.off")]
pub fn off<E>(event: E, listener: &'static <Process as Emits<E>>::Listener)
where
    Process: Emits<E>,
{
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "process.removeListener")]
pub fn remove_listener<E>(event: E, listener: &'static <Process as Emits<E>>::Listener)
where
    Process: Emits<E>,
{
    unreachable!()
}

/// `process.emit(event, ...args)`: call `event`'s listeners, as if the
/// process had.
#[cfg_attr(rust_js, rust_js::link_name = "process.emit")]
#[cfg_attr(rust_js, rust_js::variadic)]
pub fn emit<E>(event: E, args: <Process as Emits<E>>::Args) -> bool
where
    Process: Emits<E>,
{
    unreachable!()
}

/// `process.listeners(event)`.
#[cfg_attr(rust_js, rust_js::link_name = "process.listeners")]
pub fn listeners<E>(event: E) -> Vec<&'static <Process as Emits<E>>::Listener>
where
    Process: Emits<E>,
{
    unreachable!()
}

/// `process.listenerCount(event)`.
#[cfg_attr(rust_js, rust_js::link_name = "process.listenerCount")]
pub fn listener_count<E>(event: E) -> f64
where
    Process: Emits<E>,
{
    unreachable!()
}
