//! [`buffer`](https://nodejs.org/api/buffer.html): bytes. A [`Buffer`] is a
//! `Uint8Array` of Node's methods, and the global `Buffer`'s functions make
//! one: `Buffer::from_with_str("hi")`. Its `Blob` and `File` are the
//! globals, webapi's.

use core::marker::PhantomData;
use core::ops::Deref;

use js::{ArrayBuffer, JsObject, Uint8Array};

use crate::BufferEncoding;

pub use webapi::{Blob, File};

/// [`Buffer`](https://nodejs.org/api/buffer.html#class-buffer): bytes, a
/// `Uint8Array` of Node's methods.
#[cfg_attr(rust_js, rust_js::types = "Buffer")]
pub struct Buffer(PhantomData<JsObject>);

impl Deref for Buffer {
    type Target = Uint8Array;

    fn deref(&self) -> &Uint8Array {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const Uint8Array) }
    }
}

/// A [`Buffer`] of an `ArrayBuffer`, not a `SharedArrayBuffer`'s.
pub type NonSharedBuffer = Buffer;

/// A [`Buffer`] of any buffer.
pub type AllowSharedBuffer = Buffer;

impl Buffer {
    /// [`Buffer.from(array)`](https://nodejs.org/api/buffer.html#static-method-bufferfromarray):
    /// a buffer of `array`'s bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from(array: &[u8]) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.from(arrayBuffer)`: a view of `array_buffer`'s bytes, not a copy.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_array_buffer(array_buffer: &ArrayBuffer) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.from(arrayBuffer, byteOffset)`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_array_buffer_and_byte_offset(array_buffer: &ArrayBuffer, byte_offset: u32) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.from(arrayBuffer, byteOffset, length)`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_array_buffer_and_byte_offset_and_length(
        array_buffer: &ArrayBuffer,
        byte_offset: u32,
        length: u32,
    ) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.from(string)`](https://nodejs.org/api/buffer.html#static-method-bufferfromstring-encoding):
    /// `string`'s UTF-8 bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_str(string: &str) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.from(string, encoding)`: `string`'s bytes, read as `encoding`,
    /// a hex or base64 text say.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_str_and_encoding(string: &str, encoding: BufferEncoding) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.of(...items)`](https://nodejs.org/api/buffer.html#static-method-bufferofitems).
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.of")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn of(items: &[u8]) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.concat(list)`](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength):
    /// one buffer of `list`'s bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.concat")]
    pub fn concat(list: &[&Uint8Array]) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.concat(list, totalLength)`: of `total_length` bytes, cut or
    /// filled with zeros.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.concat")]
    pub fn concat_with_total_length(list: &[&Uint8Array], total_length: u32) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.copyBytesFrom(view)`](https://nodejs.org/api/buffer.html#static-method-buffercopybytesfromview-offset-length):
    /// a copy of a typed array's bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.copyBytesFrom")]
    pub fn copy_bytes_from(view: impl IntoTypedArray) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.copyBytesFrom(view, offset)`: from its element `offset`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.copyBytesFrom")]
    pub fn copy_bytes_from_with_offset(view: impl IntoTypedArray, offset: u32) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.copyBytesFrom(view, offset, length)`: `length` of its elements.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.copyBytesFrom")]
    pub fn copy_bytes_from_with_offset_and_length(view: impl IntoTypedArray, offset: u32, length: u32) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.alloc(size)`](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding):
    /// `size` zero bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.alloc")]
    pub fn alloc(size: u32) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.alloc(size, fill)`: `size` bytes, `fill`'s repeated.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.alloc")]
    pub fn alloc_with_fill(size: u32, fill: impl IntoBufferFill) -> &'static Buffer {
        unreachable!()
    }

    /// `Buffer.alloc(size, fill, encoding)`: of a text `fill`, read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.alloc")]
    pub fn alloc_with_fill_and_encoding(size: u32, fill: impl IntoBufferFill, encoding: BufferEncoding) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.allocUnsafe(size)`](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafesize):
    /// `size` bytes, not zeroed: what was there.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.allocUnsafe")]
    pub fn alloc_unsafe(size: u32) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.allocUnsafeSlow(size)`](https://nodejs.org/api/buffer.html#static-method-bufferallocunsafeslowsize):
    /// [`alloc_unsafe`](Buffer::alloc_unsafe), not of the shared pool.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.allocUnsafeSlow")]
    pub fn alloc_unsafe_slow(size: u32) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.isBuffer(obj)`](https://nodejs.org/api/buffer.html#static-method-bufferisbufferobj).
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.isBuffer")]
    pub fn is_buffer<T: ?Sized>(obj: &T) -> bool {
        unreachable!()
    }

    /// [`Buffer.isEncoding(encoding)`](https://nodejs.org/api/buffer.html#static-method-bufferisencodingencoding):
    /// whether `encoding` names a [`BufferEncoding`].
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.isEncoding")]
    pub fn is_encoding(encoding: &str) -> bool {
        unreachable!()
    }

    /// [`Buffer.byteLength(string)`](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding):
    /// how many bytes `string` is, as UTF-8, or a buffer's.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.byteLength")]
    pub fn byte_length(string: impl IntoStrOrBufferSource) -> u32 {
        unreachable!()
    }

    /// `Buffer.byteLength(string, encoding)`: as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.byteLength")]
    pub fn byte_length_with_encoding(string: impl IntoStrOrBufferSource, encoding: BufferEncoding) -> u32 {
        unreachable!()
    }

    /// [`Buffer.compare(buf1, buf2)`](https://nodejs.org/api/buffer.html#static-method-buffercomparebuf1-buf2):
    /// -1, 0 or 1, as `buf1` sorts before, as, or after `buf2`.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.compare")]
    pub fn compare_buffers(buf1: &Uint8Array, buf2: &Uint8Array) -> i32 {
        unreachable!()
    }

    /// [`Buffer.poolSize`](https://nodejs.org/api/buffer.html#class-property-bufferpoolsize):
    /// the bytes of the pool [`alloc_unsafe`](Buffer::alloc_unsafe) shares.
    #[cfg_attr(rust_js, rust_js::link_name = "get Buffer.poolSize")]
    pub fn pool_size() -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set Buffer.poolSize")]
    pub fn set_pool_size(value: u32) {
        unreachable!()
    }

    /// [`buf.write(string)`](https://nodejs.org/api/buffer.html#bufwritestring-offset-length-encoding):
    /// write `string`'s UTF-8 at 0, and give how many bytes were.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write(&self, string: &str) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_encoding(&self, string: &str, encoding: BufferEncoding) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset(&self, string: &str, offset: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset_and_encoding(&self, string: &str, offset: u32, encoding: BufferEncoding) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset_and_length(&self, string: &str, offset: u32, length: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset_and_length_and_encoding(
        &self,
        string: &str,
        offset: u32,
        length: u32,
        encoding: BufferEncoding,
    ) -> u32 {
        unreachable!()
    }

    /// [`buf.toString()`](https://nodejs.org/api/buffer.html#buftostringencoding-start-end):
    /// its bytes as UTF-8 text.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }

    /// `buf.toString(encoding)`: as `encoding`, `Hex` or `Base64` say.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string_with_encoding(&self, encoding: BufferEncoding) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string_with_encoding_and_start(&self, encoding: BufferEncoding, start: u32) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string_with_encoding_and_start_and_end(&self, encoding: BufferEncoding, start: u32, end: u32) -> String {
        unreachable!()
    }

    /// [`buf.toJSON()`](https://nodejs.org/api/buffer.html#buftojson):
    /// `{ type: "Buffer", data: [..] }`, as `JSON.stringify` writes it.
    #[cfg_attr(rust_js, rust_js::link_name = "toJSON")]
    pub fn to_json(&self) -> BufferJson {
        unreachable!()
    }

    /// [`buf.equals(otherBuffer)`](https://nodejs.org/api/buffer.html#bufequalsotherbuffer):
    /// whether it has the same bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "equals")]
    pub fn equals(&self, other_buffer: &Uint8Array) -> bool {
        unreachable!()
    }

    /// [`buf.compare(target)`](https://nodejs.org/api/buffer.html#bufcomparetarget-targetstart-targetend-sourcestart-sourceend):
    /// -1, 0 or 1, as it sorts before, as, or after `target`.
    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare(&self, target: &Uint8Array) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare_with_target_start(&self, target: &Uint8Array, target_start: u32) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare_with_target_start_and_target_end(&self, target: &Uint8Array, target_start: u32, target_end: u32) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare_with_target_start_and_target_end_and_source_start(
        &self,
        target: &Uint8Array,
        target_start: u32,
        target_end: u32,
        source_start: u32,
    ) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare_with_target_start_and_target_end_and_source_start_and_source_end(
        &self,
        target: &Uint8Array,
        target_start: u32,
        target_end: u32,
        source_start: u32,
        source_end: u32,
    ) -> i32 {
        unreachable!()
    }

    /// [`buf.copy(target)`](https://nodejs.org/api/buffer.html#bufcopytarget-targetstart-sourcestart-sourceend):
    /// copy its bytes into `target`, and give how many were.
    #[cfg_attr(rust_js, rust_js::link_name = "copy")]
    pub fn copy(&self, target: &Uint8Array) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "copy")]
    pub fn copy_with_target_start(&self, target: &Uint8Array, target_start: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "copy")]
    pub fn copy_with_target_start_and_source_start(&self, target: &Uint8Array, target_start: u32, source_start: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "copy")]
    pub fn copy_with_target_start_and_source_start_and_source_end(
        &self,
        target: &Uint8Array,
        target_start: u32,
        source_start: u32,
        source_end: u32,
    ) -> u32 {
        unreachable!()
    }

    /// [`buf.subarray()`](https://nodejs.org/api/buffer.html#bufsubarraystart-end):
    /// a view of all its bytes, not a copy.
    #[cfg_attr(rust_js, rust_js::link_name = "subarray")]
    pub fn subarray(&self) -> &'static Buffer {
        unreachable!()
    }

    /// `buf.subarray(start)`: a view of its bytes from `start`.
    #[cfg_attr(rust_js, rust_js::link_name = "subarray")]
    pub fn subarray_with_start(&self, start: i32) -> &'static Buffer {
        unreachable!()
    }

    /// `buf.subarray(start, end)`: a view of its bytes from `start` to `end`.
    #[cfg_attr(rust_js, rust_js::link_name = "subarray")]
    pub fn subarray_with_start_and_end(&self, start: i32, end: i32) -> &'static Buffer {
        unreachable!()
    }

    /// [`buf.slice()`](https://nodejs.org/api/buffer.html#bufslicestart-end):
    /// [`subarray`](Buffer::subarray), a view. Deprecated: it isn't a
    /// `Uint8Array`'s `slice`, a copy.
    #[cfg_attr(rust_js, rust_js::link_name = "slice")]
    pub fn slice(&self) -> &'static Buffer {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "slice")]
    pub fn slice_with_start(&self, start: i32) -> &'static Buffer {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "slice")]
    pub fn slice_with_start_and_end(&self, start: i32, end: i32) -> &'static Buffer {
        unreachable!()
    }

    /// [`buf.reverse()`](https://nodejs.org/api/buffer.html): its bytes, in
    /// place, the other way.
    #[cfg_attr(rust_js, rust_js::link_name = "reverse")]
    pub fn reverse(&self) -> &Self {
        unreachable!()
    }

    /// [`buf.swap16()`](https://nodejs.org/api/buffer.html#bufswap16): each
    /// two bytes', in place.
    #[cfg_attr(rust_js, rust_js::link_name = "swap16")]
    pub fn swap16(&self) -> &Self {
        unreachable!()
    }

    /// [`buf.swap32()`](https://nodejs.org/api/buffer.html#bufswap32).
    #[cfg_attr(rust_js, rust_js::link_name = "swap32")]
    pub fn swap32(&self) -> &Self {
        unreachable!()
    }

    /// [`buf.swap64()`](https://nodejs.org/api/buffer.html#bufswap64).
    #[cfg_attr(rust_js, rust_js::link_name = "swap64")]
    pub fn swap64(&self) -> &Self {
        unreachable!()
    }

    /// [`buf.fill(value)`](https://nodejs.org/api/buffer.html#buffillvalue-offset-end-encoding):
    /// each byte of `value`'s, repeated.
    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill(&self, value: impl IntoBufferFill) -> &Self {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_offset(&self, value: impl IntoBufferFill, offset: u32) -> &Self {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_offset_and_end(&self, value: impl IntoBufferFill, offset: u32, end: u32) -> &Self {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_offset_and_end_and_encoding(
        &self,
        value: impl IntoBufferFill,
        offset: u32,
        end: u32,
        encoding: BufferEncoding,
    ) -> &Self {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_offset_and_encoding(&self, value: impl IntoBufferFill, offset: u32, encoding: BufferEncoding) -> &Self {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> &Self {
        unreachable!()
    }

    /// [`buf.indexOf(value)`](https://nodejs.org/api/buffer.html#bufindexofvalue-byteoffset-encoding):
    /// where `value`'s bytes first are, or -1.
    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of(&self, value: impl IntoBufferFill) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of_with_byte_offset(&self, value: impl IntoBufferFill, byte_offset: i32) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of_with_byte_offset_and_encoding(&self, value: impl IntoBufferFill, byte_offset: i32, encoding: BufferEncoding) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> i32 {
        unreachable!()
    }

    /// [`buf.lastIndexOf(value)`](https://nodejs.org/api/buffer.html#buflastindexofvalue-byteoffset-encoding):
    /// where `value`'s bytes last are, or -1.
    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of(&self, value: impl IntoBufferFill) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of_with_byte_offset(&self, value: impl IntoBufferFill, byte_offset: i32) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of_with_byte_offset_and_encoding(
        &self,
        value: impl IntoBufferFill,
        byte_offset: i32,
        encoding: BufferEncoding,
    ) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> i32 {
        unreachable!()
    }

    /// [`buf.includes(value)`](https://nodejs.org/api/buffer.html#bufincludesvalue-byteoffset-encoding):
    /// whether `value`'s bytes are in it.
    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes(&self, value: impl IntoBufferFill) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes_with_byte_offset(&self, value: impl IntoBufferFill, byte_offset: i32) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes_with_byte_offset_and_encoding(&self, value: impl IntoBufferFill, byte_offset: i32, encoding: BufferEncoding) -> bool {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> bool {
        unreachable!()
    }

    /// [`buf.readUint8()`](https://nodejs.org/api/buffer.html#bufreaduint8offset): an unsigned byte at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint8")]
    pub fn read_uint8(&self) -> u8 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readUint8")]
    pub fn read_uint8_with_offset(&self, offset: u32) -> u8 {
        unreachable!()
    }

    /// [`buf.readUint16LE()`](https://nodejs.org/api/buffer.html#bufreaduint16leoffset): a little-endian `u16` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint16LE")]
    pub fn read_uint16_le(&self) -> u16 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readUint16LE")]
    pub fn read_uint16_le_with_offset(&self, offset: u32) -> u16 {
        unreachable!()
    }

    /// [`buf.readUint16BE()`](https://nodejs.org/api/buffer.html#bufreaduint16beoffset): a big-endian `u16` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint16BE")]
    pub fn read_uint16_be(&self) -> u16 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readUint16BE")]
    pub fn read_uint16_be_with_offset(&self, offset: u32) -> u16 {
        unreachable!()
    }

    /// [`buf.readUint32LE()`](https://nodejs.org/api/buffer.html#bufreaduint32leoffset): a little-endian `u32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint32LE")]
    pub fn read_uint32_le(&self) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readUint32LE")]
    pub fn read_uint32_le_with_offset(&self, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readUint32BE()`](https://nodejs.org/api/buffer.html#bufreaduint32beoffset): a big-endian `u32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint32BE")]
    pub fn read_uint32_be(&self) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readUint32BE")]
    pub fn read_uint32_be_with_offset(&self, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readInt8()`](https://nodejs.org/api/buffer.html#bufreadint8offset): a signed byte at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt8")]
    pub fn read_int8(&self) -> i8 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readInt8")]
    pub fn read_int8_with_offset(&self, offset: u32) -> i8 {
        unreachable!()
    }

    /// [`buf.readInt16LE()`](https://nodejs.org/api/buffer.html#bufreadint16leoffset): a little-endian `i16` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt16LE")]
    pub fn read_int16_le(&self) -> i16 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readInt16LE")]
    pub fn read_int16_le_with_offset(&self, offset: u32) -> i16 {
        unreachable!()
    }

    /// [`buf.readInt16BE()`](https://nodejs.org/api/buffer.html#bufreadint16beoffset): a big-endian `i16` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt16BE")]
    pub fn read_int16_be(&self) -> i16 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readInt16BE")]
    pub fn read_int16_be_with_offset(&self, offset: u32) -> i16 {
        unreachable!()
    }

    /// [`buf.readInt32LE()`](https://nodejs.org/api/buffer.html#bufreadint32leoffset): a little-endian `i32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt32LE")]
    pub fn read_int32_le(&self) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readInt32LE")]
    pub fn read_int32_le_with_offset(&self, offset: u32) -> i32 {
        unreachable!()
    }

    /// [`buf.readInt32BE()`](https://nodejs.org/api/buffer.html#bufreadint32beoffset): a big-endian `i32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt32BE")]
    pub fn read_int32_be(&self) -> i32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readInt32BE")]
    pub fn read_int32_be_with_offset(&self, offset: u32) -> i32 {
        unreachable!()
    }

    /// [`buf.readFloatLE()`](https://nodejs.org/api/buffer.html#bufreadfloatleoffset): a little-endian `f32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readFloatLE")]
    pub fn read_float_le(&self) -> f32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readFloatLE")]
    pub fn read_float_le_with_offset(&self, offset: u32) -> f32 {
        unreachable!()
    }

    /// [`buf.readFloatBE()`](https://nodejs.org/api/buffer.html#bufreadfloatbeoffset): a big-endian `f32` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readFloatBE")]
    pub fn read_float_be(&self) -> f32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readFloatBE")]
    pub fn read_float_be_with_offset(&self, offset: u32) -> f32 {
        unreachable!()
    }

    /// [`buf.readDoubleLE()`](https://nodejs.org/api/buffer.html#bufreaddoubleleoffset): a little-endian `f64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleLE")]
    pub fn read_double_le(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleLE")]
    pub fn read_double_le_with_offset(&self, offset: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.readDoubleBE()`](https://nodejs.org/api/buffer.html#bufreaddoublebeoffset): a big-endian `f64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleBE")]
    pub fn read_double_be(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleBE")]
    pub fn read_double_be_with_offset(&self, offset: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.readBigInt64LE()`](https://nodejs.org/api/buffer.html#bufreadbigint64leoffset): a little-endian `i64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64LE")]
    pub fn read_big_int64_le(&self) -> i64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64LE")]
    pub fn read_big_int64_le_with_offset(&self, offset: u32) -> i64 {
        unreachable!()
    }

    /// [`buf.readBigInt64BE()`](https://nodejs.org/api/buffer.html#bufreadbigint64beoffset): a big-endian `i64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64BE")]
    pub fn read_big_int64_be(&self) -> i64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64BE")]
    pub fn read_big_int64_be_with_offset(&self, offset: u32) -> i64 {
        unreachable!()
    }

    /// [`buf.readBigUint64LE()`](https://nodejs.org/api/buffer.html#bufreadbiguint64leoffset): a little-endian `u64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64LE")]
    pub fn read_big_uint64_le(&self) -> u64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64LE")]
    pub fn read_big_uint64_le_with_offset(&self, offset: u32) -> u64 {
        unreachable!()
    }

    /// [`buf.readBigUint64BE()`](https://nodejs.org/api/buffer.html#bufreadbiguint64beoffset): a big-endian `u64` at 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64BE")]
    pub fn read_big_uint64_be(&self) -> u64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64BE")]
    pub fn read_big_uint64_be_with_offset(&self, offset: u32) -> u64 {
        unreachable!()
    }

    /// [`buf.writeUint8(value)`](https://nodejs.org/api/buffer.html#bufwriteuint8value-offset): write an unsigned byte at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint8")]
    pub fn write_uint8(&self, value: u8) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeUint8")]
    pub fn write_uint8_with_offset(&self, value: u8, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint16LE(value)`](https://nodejs.org/api/buffer.html#bufwriteuint16levalue-offset): write a little-endian `u16` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16LE")]
    pub fn write_uint16_le(&self, value: u16) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16LE")]
    pub fn write_uint16_le_with_offset(&self, value: u16, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint16BE(value)`](https://nodejs.org/api/buffer.html#bufwriteuint16bevalue-offset): write a big-endian `u16` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16BE")]
    pub fn write_uint16_be(&self, value: u16) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16BE")]
    pub fn write_uint16_be_with_offset(&self, value: u16, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint32LE(value)`](https://nodejs.org/api/buffer.html#bufwriteuint32levalue-offset): write a little-endian `u32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32LE")]
    pub fn write_uint32_le(&self, value: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32LE")]
    pub fn write_uint32_le_with_offset(&self, value: u32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint32BE(value)`](https://nodejs.org/api/buffer.html#bufwriteuint32bevalue-offset): write a big-endian `u32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32BE")]
    pub fn write_uint32_be(&self, value: u32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32BE")]
    pub fn write_uint32_be_with_offset(&self, value: u32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt8(value)`](https://nodejs.org/api/buffer.html#bufwriteint8value-offset): write a signed byte at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt8")]
    pub fn write_int8(&self, value: i8) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeInt8")]
    pub fn write_int8_with_offset(&self, value: i8, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt16LE(value)`](https://nodejs.org/api/buffer.html#bufwriteint16levalue-offset): write a little-endian `i16` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16LE")]
    pub fn write_int16_le(&self, value: i16) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16LE")]
    pub fn write_int16_le_with_offset(&self, value: i16, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt16BE(value)`](https://nodejs.org/api/buffer.html#bufwriteint16bevalue-offset): write a big-endian `i16` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16BE")]
    pub fn write_int16_be(&self, value: i16) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16BE")]
    pub fn write_int16_be_with_offset(&self, value: i16, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt32LE(value)`](https://nodejs.org/api/buffer.html#bufwriteint32levalue-offset): write a little-endian `i32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32LE")]
    pub fn write_int32_le(&self, value: i32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32LE")]
    pub fn write_int32_le_with_offset(&self, value: i32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt32BE(value)`](https://nodejs.org/api/buffer.html#bufwriteint32bevalue-offset): write a big-endian `i32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32BE")]
    pub fn write_int32_be(&self, value: i32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32BE")]
    pub fn write_int32_be_with_offset(&self, value: i32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeFloatLE(value)`](https://nodejs.org/api/buffer.html#bufwritefloatlevalue-offset): write a little-endian `f32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatLE")]
    pub fn write_float_le(&self, value: f32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatLE")]
    pub fn write_float_le_with_offset(&self, value: f32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeFloatBE(value)`](https://nodejs.org/api/buffer.html#bufwritefloatbevalue-offset): write a big-endian `f32` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatBE")]
    pub fn write_float_be(&self, value: f32) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatBE")]
    pub fn write_float_be_with_offset(&self, value: f32, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeDoubleLE(value)`](https://nodejs.org/api/buffer.html#bufwritedoublelevalue-offset): write a little-endian `f64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleLE")]
    pub fn write_double_le(&self, value: f64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleLE")]
    pub fn write_double_le_with_offset(&self, value: f64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeDoubleBE(value)`](https://nodejs.org/api/buffer.html#bufwritedoublebevalue-offset): write a big-endian `f64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleBE")]
    pub fn write_double_be(&self, value: f64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleBE")]
    pub fn write_double_be_with_offset(&self, value: f64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigInt64LE(value)`](https://nodejs.org/api/buffer.html#bufwritebigint64levalue-offset): write a little-endian `i64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64LE")]
    pub fn write_big_int64_le(&self, value: i64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64LE")]
    pub fn write_big_int64_le_with_offset(&self, value: i64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigInt64BE(value)`](https://nodejs.org/api/buffer.html#bufwritebigint64bevalue-offset): write a big-endian `i64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64BE")]
    pub fn write_big_int64_be(&self, value: i64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64BE")]
    pub fn write_big_int64_be_with_offset(&self, value: i64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigUint64LE(value)`](https://nodejs.org/api/buffer.html#bufwritebiguint64levalue-offset): write a little-endian `u64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64LE")]
    pub fn write_big_uint64_le(&self, value: u64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64LE")]
    pub fn write_big_uint64_le_with_offset(&self, value: u64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigUint64BE(value)`](https://nodejs.org/api/buffer.html#bufwritebiguint64bevalue-offset): write a big-endian `u64` at 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64BE")]
    pub fn write_big_uint64_be(&self, value: u64) -> u32 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64BE")]
    pub fn write_big_uint64_be_with_offset(&self, value: u64, offset: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readUintLE(offset, byteLength)`](https://nodejs.org/api/buffer.html#bufreaduintleoffset-bytelength): an unsigned integer of `byte_length` bytes, up to 6, little-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "readUintLE")]
    pub fn read_uint_le(&self, offset: u32, byte_length: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.writeUintLE(value, offset, byteLength)`](https://nodejs.org/api/buffer.html#bufwriteuintlevalue-offset-bytelength): write an unsigned integer of `byte_length` bytes, up to 6, little-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUintLE")]
    pub fn write_uint_le(&self, value: f64, offset: u32, byte_length: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readUintBE(offset, byteLength)`](https://nodejs.org/api/buffer.html#bufreaduintbeoffset-bytelength): an unsigned integer of `byte_length` bytes, up to 6, big-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "readUintBE")]
    pub fn read_uint_be(&self, offset: u32, byte_length: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.writeUintBE(value, offset, byteLength)`](https://nodejs.org/api/buffer.html#bufwriteuintbevalue-offset-bytelength): write an unsigned integer of `byte_length` bytes, up to 6, big-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUintBE")]
    pub fn write_uint_be(&self, value: f64, offset: u32, byte_length: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readIntLE(offset, byteLength)`](https://nodejs.org/api/buffer.html#bufreadintleoffset-bytelength): a signed integer of `byte_length` bytes, up to 6, little-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "readIntLE")]
    pub fn read_int_le(&self, offset: u32, byte_length: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.writeIntLE(value, offset, byteLength)`](https://nodejs.org/api/buffer.html#bufwriteintlevalue-offset-bytelength): write a signed integer of `byte_length` bytes, up to 6, little-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "writeIntLE")]
    pub fn write_int_le(&self, value: f64, offset: u32, byte_length: u32) -> u32 {
        unreachable!()
    }

    /// [`buf.readIntBE(offset, byteLength)`](https://nodejs.org/api/buffer.html#bufreadintbeoffset-bytelength): a signed integer of `byte_length` bytes, up to 6, big-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "readIntBE")]
    pub fn read_int_be(&self, offset: u32, byte_length: u32) -> f64 {
        unreachable!()
    }

    /// [`buf.writeIntBE(value, offset, byteLength)`](https://nodejs.org/api/buffer.html#bufwriteintbevalue-offset-bytelength): write a signed integer of `byte_length` bytes, up to 6, big-endian.
    #[cfg_attr(rust_js, rust_js::link_name = "writeIntBE")]
    pub fn write_int_be(&self, value: f64, offset: u32, byte_length: u32) -> u32 {
        unreachable!()
    }

}

/// What [`Buffer::to_json`] gives: `{ type: "Buffer", data }`.
pub struct BufferJson {
    /// `"Buffer"`.
    #[cfg_attr(rust_js, rust_js::name = "type")]
    pub type_: String,
    pub data: Vec<u8>,
}

/// [`SlowBuffer(size)`](https://nodejs.org/api/buffer.html#class-slowbuffer):
/// [`Buffer::alloc_unsafe_slow`]. Deprecated, and gone after Node 24.
#[cfg_attr(rust_js, rust_js::link_name = "new buffer#SlowBuffer")]
pub fn slow_buffer(size: u32) -> &'static Buffer {
    unreachable!()
}

/// [`buffer.isUtf8(input)`](https://nodejs.org/api/buffer.html#bufferisutf8input):
/// whether `input`'s bytes are UTF-8.
#[cfg_attr(rust_js, rust_js::link_name = "buffer#isUtf8")]
pub fn is_utf8(input: impl IntoTypedArrayOrArrayBuffer) -> bool {
    unreachable!()
}

/// [`buffer.isAscii(input)`](https://nodejs.org/api/buffer.html#bufferisasciiinput):
/// whether `input`'s bytes are ASCII.
#[cfg_attr(rust_js, rust_js::link_name = "buffer#isAscii")]
pub fn is_ascii(input: impl IntoTypedArrayOrArrayBuffer) -> bool {
    unreachable!()
}

unsafe extern "Rust" {
    /// [`buffer.INSPECT_MAX_BYTES`](https://nodejs.org/api/buffer.html#bufferinspect_max_bytes):
    /// how many bytes a buffer shows, 50.
    #[link_name = "buffer#INSPECT_MAX_BYTES"]
    pub safe static INSPECT_MAX_BYTES: f64;

    /// [`buffer.kMaxLength`](https://nodejs.org/api/buffer.html#bufferkmaxlength):
    /// the most bytes a buffer may be.
    #[link_name = "buffer#kMaxLength"]
    #[allow(non_upper_case_globals)]
    pub safe static kMaxLength: f64;

    /// [`buffer.kStringMaxLength`](https://nodejs.org/api/buffer.html#bufferkstringmaxlength):
    /// the longest text, in UTF-16 units.
    #[link_name = "buffer#kStringMaxLength"]
    #[allow(non_upper_case_globals)]
    pub safe static kStringMaxLength: f64;

    /// [`buffer.constants`](https://nodejs.org/api/buffer.html#bufferconstants).
    #[link_name = "buffer#constants"]
    pub safe static constants: &'static BufferConstants;

    /// [`buffer.transcode(source, fromEnc, toEnc)`](https://nodejs.org/api/buffer.html#buffertranscodesource-fromenc-toenc):
    /// `source`'s text, as `from_enc`, in `to_enc`.
    #[link_name = "buffer#transcode"]
    pub safe fn transcode(source: &Uint8Array, from_enc: TranscodeEncoding, to_enc: TranscodeEncoding) -> &'static Buffer;

    /// [`buffer.resolveObjectURL(id)`](https://nodejs.org/api/buffer.html#bufferresolveobjecturlid):
    /// the `Blob` a `blob:` URL `URL.createObjectURL` made is, or `None`.
    #[link_name = "buffer#resolveObjectURL"]
    pub safe fn resolve_object_url(id: &str) -> Option<&'static Blob>;
}

/// [`buffer.constants`](constants).
pub struct BufferConstants(PhantomData<JsObject>);

impl BufferConstants {
    #[cfg_attr(rust_js, rust_js::link_name = "get MAX_LENGTH")]
    pub fn max_length(&self) -> f64 {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get MAX_STRING_LENGTH")]
    pub fn max_string_length(&self) -> f64 {
        unreachable!()
    }
}

/// What [`transcode`] reads and writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TranscodeEncoding {
    #[cfg_attr(rust_js, rust_js::name = "ascii")]
    Ascii,
    #[cfg_attr(rust_js, rust_js::name = "utf8")]
    Utf8,
    #[cfg_attr(rust_js, rust_js::name = "utf-8")]
    Utf8Dash,
    #[cfg_attr(rust_js, rust_js::name = "utf16le")]
    Utf16le,
    #[cfg_attr(rust_js, rust_js::name = "utf-16le")]
    Utf16leDash,
    #[cfg_attr(rust_js, rust_js::name = "ucs2")]
    Ucs2,
    #[cfg_attr(rust_js, rust_js::name = "ucs-2")]
    Ucs2Dash,
    #[cfg_attr(rust_js, rust_js::name = "latin1")]
    Latin1,
    #[cfg_attr(rust_js, rust_js::name = "binary")]
    Binary,
}

/// A `Blob`'s options, webapi's `BlobPropertyBag`.
pub type BlobOptions<'a> = webapi::BlobPropertyBag<'a>;

/// A `File`'s options, webapi's `FilePropertyBag`.
pub type FileOptions<'a> = webapi::FilePropertyBag<'a>;

/// What a `string | Uint8Array | number` parameter, a buffer's fill or
/// what it searches for, takes: each as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a `string | Uint8Array | number`")]
#[cfg_attr(rust_js, rust_js::types = "string | Uint8Array | number")]
pub trait IntoBufferFill: sealed::Sealed {}
impl IntoBufferFill for &str {}
impl IntoBufferFill for &Uint8Array {}
impl IntoBufferFill for &Buffer {}
impl IntoBufferFill for u8 {}

/// What a `string | ArrayBufferView | ArrayBuffer` parameter takes: each
/// as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a text nor bytes")]
#[cfg_attr(rust_js, rust_js::types = "string | NodeJS.ArrayBufferView | ArrayBufferLike")]
pub trait IntoStrOrBufferSource: sealed::Sealed {}
impl IntoStrOrBufferSource for &str {}
impl IntoStrOrBufferSource for &Uint8Array {}
impl IntoStrOrBufferSource for &Buffer {}
impl IntoStrOrBufferSource for &ArrayBuffer {}

/// What a `NodeJS.TypedArray` parameter takes: each typed array, a
/// [`Buffer`] among them, as it is (ADR 0229).
#[diagnostic::on_unimplemented(message = "`{Self}` is not a typed array")]
#[cfg_attr(rust_js, rust_js::types = "NodeJS.TypedArray")]
pub trait IntoTypedArray: sealed::Sealed {}
impl IntoTypedArray for &Buffer {}
impl IntoTypedArray for &js::Int8Array {}
impl IntoTypedArray for &js::Uint8Array {}
impl IntoTypedArray for &js::Uint8ClampedArray {}
impl IntoTypedArray for &js::Int16Array {}
impl IntoTypedArray for &js::Uint16Array {}
impl IntoTypedArray for &js::Int32Array {}
impl IntoTypedArray for &js::Uint32Array {}
impl IntoTypedArray for &js::Float32Array {}
impl IntoTypedArray for &js::Float64Array {}
impl IntoTypedArray for &js::BigInt64Array {}
impl IntoTypedArray for &js::BigUint64Array {}

/// What an `ArrayBuffer | NodeJS.TypedArray` parameter takes.
#[diagnostic::on_unimplemented(message = "`{Self}` is not an `ArrayBuffer` nor a typed array")]
#[cfg_attr(rust_js, rust_js::types = "ArrayBuffer | NodeJS.TypedArray")]
pub trait IntoTypedArrayOrArrayBuffer: sealed::Sealed {}
impl IntoTypedArrayOrArrayBuffer for &ArrayBuffer {}
impl IntoTypedArrayOrArrayBuffer for &Buffer {}
impl IntoTypedArrayOrArrayBuffer for &js::Int8Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Uint8Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Uint8ClampedArray {}
impl IntoTypedArrayOrArrayBuffer for &js::Int16Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Uint16Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Int32Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Uint32Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Float32Array {}
impl IntoTypedArrayOrArrayBuffer for &js::Float64Array {}
impl IntoTypedArrayOrArrayBuffer for &js::BigInt64Array {}
impl IntoTypedArrayOrArrayBuffer for &js::BigUint64Array {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for &str {}
    impl Sealed for &super::Buffer {}
    impl Sealed for &js::ArrayBuffer {}
    impl Sealed for u8 {}
    impl Sealed for &js::Int8Array {}
    impl Sealed for &js::Uint8Array {}
    impl Sealed for &js::Uint8ClampedArray {}
    impl Sealed for &js::Int16Array {}
    impl Sealed for &js::Uint16Array {}
    impl Sealed for &js::Int32Array {}
    impl Sealed for &js::Uint32Array {}
    impl Sealed for &js::Float32Array {}
    impl Sealed for &js::Float64Array {}
    impl Sealed for &js::BigInt64Array {}
    impl Sealed for &js::BigUint64Array {}
}
