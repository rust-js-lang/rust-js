//! [`buffer`](https://nodejs.org/api/buffer.html): bytes. A [`Buffer`] is a
//! `Uint8Array` of Node's methods, and the global `Buffer`'s functions make
//! one: `Buffer::from_with_str("hi", None)`. Its `Blob` and `File` are the
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

    /// [`Buffer.from(arrayBuffer, byteOffset, length)`](https://nodejs.org/api/buffer.html#static-method-bufferfromarraybuffer-byteoffset-length):
    /// a view of `array_buffer`'s bytes, not a copy, from `byte_offset`, or
    /// 0, `length` of them, or to its end.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_array_buffer(array_buffer: &ArrayBuffer, byte_offset: Option<u32>, length: Option<u32>) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.from(string, encoding)`](https://nodejs.org/api/buffer.html#static-method-bufferfromstring-encoding):
    /// `string`'s bytes, read as `encoding`, a hex or base64 text say, or
    /// as UTF-8.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.from")]
    pub fn from_with_str(string: &str, encoding: Option<BufferEncoding>) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.of(...items)`](https://nodejs.org/api/buffer.html#static-method-bufferofitems).
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.of")]
    #[cfg_attr(rust_js, rust_js::variadic)]
    pub fn of(items: &[u8]) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.concat(list, totalLength)`](https://nodejs.org/api/buffer.html#static-method-bufferconcatlist-totallength):
    /// one buffer of `list`'s bytes, of `total_length` bytes, cut or filled
    /// with zeros, where it's given.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.concat")]
    pub fn concat(list: &[&Uint8Array], total_length: Option<u32>) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.copyBytesFrom(view, offset, length)`](https://nodejs.org/api/buffer.html#static-method-buffercopybytesfromview-offset-length):
    /// a copy of a typed array's bytes, from its element `offset`, or 0,
    /// `length` of its elements, or to its end.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.copyBytesFrom")]
    pub fn copy_bytes_from(view: impl IntoTypedArray, offset: Option<u32>, length: Option<u32>) -> &'static Buffer {
        unreachable!()
    }

    /// [`Buffer.alloc(size, fill, encoding)`](https://nodejs.org/api/buffer.html#static-method-bufferallocsize-fill-encoding):
    /// `size` bytes, `fill`'s repeated, a text one read as `encoding`, or
    /// zeros.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.alloc")]
    pub fn alloc(size: u32, fill: Option<&dyn IntoBufferFill>, encoding: Option<BufferEncoding>) -> &'static Buffer {
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

    /// [`Buffer.byteLength(string, encoding)`](https://nodejs.org/api/buffer.html#static-method-bufferbytelengthstring-encoding):
    /// how many bytes `string` is, as `encoding`, or as UTF-8, or a buffer's.
    #[cfg_attr(rust_js, rust_js::link_name = "Buffer.byteLength")]
    pub fn byte_length(string: impl IntoStrOrBufferSource, encoding: Option<BufferEncoding>) -> u32 {
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

    /// [`buf.write(string, encoding)`](https://nodejs.org/api/buffer.html#bufwritestring-offset-length-encoding):
    /// write `string`, as `encoding`, or UTF-8, at 0, and give how many
    /// bytes were.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write(&self, string: &str, encoding: Option<BufferEncoding>) -> u32 {
        unreachable!()
    }

    /// `buf.write(string, offset, encoding)`: at `offset`.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset(&self, string: &str, offset: u32, encoding: Option<BufferEncoding>) -> u32 {
        unreachable!()
    }

    /// `buf.write(string, offset, length, encoding)`: at `offset`, at most
    /// `length` bytes.
    #[cfg_attr(rust_js, rust_js::link_name = "write")]
    pub fn write_with_offset_and_length(&self, string: &str, offset: u32, length: u32, encoding: Option<BufferEncoding>) -> u32 {
        unreachable!()
    }

    /// [`buf.toString(encoding, start, end)`](https://nodejs.org/api/buffer.html#buftostringencoding-start-end):
    /// its bytes, from `start` to `end`, or all of them, as `encoding`'s
    /// text, `Hex` or `Base64` say, or UTF-8.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self, encoding: Option<BufferEncoding>, start: Option<u32>, end: Option<u32>) -> String {
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

    /// [`buf.compare(target, targetStart, targetEnd, sourceStart, sourceEnd)`](https://nodejs.org/api/buffer.html#bufcomparetarget-targetstart-targetend-sourcestart-sourceend):
    /// -1, 0 or 1, as it sorts before, as, or after `target`, each of the
    /// two from its start to its end, or whole.
    #[cfg_attr(rust_js, rust_js::link_name = "compare")]
    pub fn compare(
        &self,
        target: &Uint8Array,
        target_start: Option<u32>,
        target_end: Option<u32>,
        source_start: Option<u32>,
        source_end: Option<u32>,
    ) -> i32 {
        unreachable!()
    }

    /// [`buf.copy(target, targetStart, sourceStart, sourceEnd)`](https://nodejs.org/api/buffer.html#bufcopytarget-targetstart-sourcestart-sourceend):
    /// copy its bytes, from `source_start` to `source_end`, or all of them,
    /// into `target` at `target_start`, or 0, and give how many were.
    #[cfg_attr(rust_js, rust_js::link_name = "copy")]
    pub fn copy(&self, target: &Uint8Array, target_start: Option<u32>, source_start: Option<u32>, source_end: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.subarray(start, end)`](https://nodejs.org/api/buffer.html#bufsubarraystart-end):
    /// a view of its bytes, not a copy, from `start` to `end`, or all of them.
    #[cfg_attr(rust_js, rust_js::link_name = "subarray")]
    pub fn subarray(&self, start: Option<i32>, end: Option<i32>) -> &'static Buffer {
        unreachable!()
    }

    /// [`buf.slice(start, end)`](https://nodejs.org/api/buffer.html#bufslicestart-end):
    /// [`subarray`](Buffer::subarray), a view. Deprecated: it isn't a
    /// `Uint8Array`'s `slice`, a copy.
    #[cfg_attr(rust_js, rust_js::link_name = "slice")]
    pub fn slice(&self, start: Option<i32>, end: Option<i32>) -> &'static Buffer {
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

    /// [`buf.fill(value, offset, end, encoding)`](https://nodejs.org/api/buffer.html#buffillvalue-offset-end-encoding):
    /// each byte, from `offset` to `end`, or all, of `value`'s, repeated, a
    /// text one read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill(&self, value: impl IntoBufferFill, offset: Option<u32>, end: Option<u32>, encoding: Option<BufferEncoding>) -> &Self {
        unreachable!()
    }

    /// `buf.fill(value, offset, encoding)`: from `offset`, a text `value`
    /// read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_offset_and_encoding(&self, value: impl IntoBufferFill, offset: u32, encoding: BufferEncoding) -> &Self {
        unreachable!()
    }

    /// `buf.fill(value, encoding)`: a text `value` read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "fill")]
    pub fn fill_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> &Self {
        unreachable!()
    }

    /// [`buf.indexOf(value, byteOffset, encoding)`](https://nodejs.org/api/buffer.html#bufindexofvalue-byteoffset-encoding):
    /// where `value`'s bytes first are, from `byte_offset`, or -1; a text
    /// one's read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of(&self, value: impl IntoBufferFill, byte_offset: Option<i32>, encoding: Option<BufferEncoding>) -> i32 {
        unreachable!()
    }

    /// `buf.indexOf(value, encoding)`: a text `value` read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "indexOf")]
    pub fn index_of_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> i32 {
        unreachable!()
    }

    /// [`buf.lastIndexOf(value, byteOffset, encoding)`](https://nodejs.org/api/buffer.html#buflastindexofvalue-byteoffset-encoding):
    /// where `value`'s bytes last are, up to `byte_offset`, or -1; a text
    /// one's read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of(&self, value: impl IntoBufferFill, byte_offset: Option<i32>, encoding: Option<BufferEncoding>) -> i32 {
        unreachable!()
    }

    /// `buf.lastIndexOf(value, encoding)`: a text `value` read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "lastIndexOf")]
    pub fn last_index_of_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> i32 {
        unreachable!()
    }

    /// [`buf.includes(value, byteOffset, encoding)`](https://nodejs.org/api/buffer.html#bufincludesvalue-byteoffset-encoding):
    /// whether `value`'s bytes are in it, from `byte_offset`; a text one's
    /// read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes(&self, value: impl IntoBufferFill, byte_offset: Option<i32>, encoding: Option<BufferEncoding>) -> bool {
        unreachable!()
    }

    /// `buf.includes(value, encoding)`: a text `value` read as `encoding`.
    #[cfg_attr(rust_js, rust_js::link_name = "includes")]
    pub fn includes_with_encoding(&self, value: impl IntoBufferFill, encoding: BufferEncoding) -> bool {
        unreachable!()
    }

    /// [`buf.readUint8(offset)`](https://nodejs.org/api/buffer.html#bufreaduint8offset): an unsigned byte at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint8")]
    pub fn read_uint8(&self, offset: Option<u32>) -> u8 {
        unreachable!()
    }

    /// [`buf.readUint16LE(offset)`](https://nodejs.org/api/buffer.html#bufreaduint16leoffset): a little-endian `u16` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint16LE")]
    pub fn read_uint16_le(&self, offset: Option<u32>) -> u16 {
        unreachable!()
    }

    /// [`buf.readUint16BE(offset)`](https://nodejs.org/api/buffer.html#bufreaduint16beoffset): a big-endian `u16` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint16BE")]
    pub fn read_uint16_be(&self, offset: Option<u32>) -> u16 {
        unreachable!()
    }

    /// [`buf.readUint32LE(offset)`](https://nodejs.org/api/buffer.html#bufreaduint32leoffset): a little-endian `u32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint32LE")]
    pub fn read_uint32_le(&self, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.readUint32BE(offset)`](https://nodejs.org/api/buffer.html#bufreaduint32beoffset): a big-endian `u32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readUint32BE")]
    pub fn read_uint32_be(&self, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.readInt8(offset)`](https://nodejs.org/api/buffer.html#bufreadint8offset): a signed byte at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt8")]
    pub fn read_int8(&self, offset: Option<u32>) -> i8 {
        unreachable!()
    }

    /// [`buf.readInt16LE(offset)`](https://nodejs.org/api/buffer.html#bufreadint16leoffset): a little-endian `i16` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt16LE")]
    pub fn read_int16_le(&self, offset: Option<u32>) -> i16 {
        unreachable!()
    }

    /// [`buf.readInt16BE(offset)`](https://nodejs.org/api/buffer.html#bufreadint16beoffset): a big-endian `i16` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt16BE")]
    pub fn read_int16_be(&self, offset: Option<u32>) -> i16 {
        unreachable!()
    }

    /// [`buf.readInt32LE(offset)`](https://nodejs.org/api/buffer.html#bufreadint32leoffset): a little-endian `i32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt32LE")]
    pub fn read_int32_le(&self, offset: Option<u32>) -> i32 {
        unreachable!()
    }

    /// [`buf.readInt32BE(offset)`](https://nodejs.org/api/buffer.html#bufreadint32beoffset): a big-endian `i32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readInt32BE")]
    pub fn read_int32_be(&self, offset: Option<u32>) -> i32 {
        unreachable!()
    }

    /// [`buf.readFloatLE(offset)`](https://nodejs.org/api/buffer.html#bufreadfloatleoffset): a little-endian `f32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readFloatLE")]
    pub fn read_float_le(&self, offset: Option<u32>) -> f32 {
        unreachable!()
    }

    /// [`buf.readFloatBE(offset)`](https://nodejs.org/api/buffer.html#bufreadfloatbeoffset): a big-endian `f32` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readFloatBE")]
    pub fn read_float_be(&self, offset: Option<u32>) -> f32 {
        unreachable!()
    }

    /// [`buf.readDoubleLE(offset)`](https://nodejs.org/api/buffer.html#bufreaddoubleleoffset): a little-endian `f64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleLE")]
    pub fn read_double_le(&self, offset: Option<u32>) -> f64 {
        unreachable!()
    }

    /// [`buf.readDoubleBE(offset)`](https://nodejs.org/api/buffer.html#bufreaddoublebeoffset): a big-endian `f64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readDoubleBE")]
    pub fn read_double_be(&self, offset: Option<u32>) -> f64 {
        unreachable!()
    }

    /// [`buf.readBigInt64LE(offset)`](https://nodejs.org/api/buffer.html#bufreadbigint64leoffset): a little-endian `i64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64LE")]
    pub fn read_big_int64_le(&self, offset: Option<u32>) -> i64 {
        unreachable!()
    }

    /// [`buf.readBigInt64BE(offset)`](https://nodejs.org/api/buffer.html#bufreadbigint64beoffset): a big-endian `i64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigInt64BE")]
    pub fn read_big_int64_be(&self, offset: Option<u32>) -> i64 {
        unreachable!()
    }

    /// [`buf.readBigUint64LE(offset)`](https://nodejs.org/api/buffer.html#bufreadbiguint64leoffset): a little-endian `u64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64LE")]
    pub fn read_big_uint64_le(&self, offset: Option<u32>) -> u64 {
        unreachable!()
    }

    /// [`buf.readBigUint64BE(offset)`](https://nodejs.org/api/buffer.html#bufreadbiguint64beoffset): a big-endian `u64` at `offset`, or 0.
    #[cfg_attr(rust_js, rust_js::link_name = "readBigUint64BE")]
    pub fn read_big_uint64_be(&self, offset: Option<u32>) -> u64 {
        unreachable!()
    }

    /// [`buf.writeUint8(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteuint8value-offset): write an unsigned byte at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint8")]
    pub fn write_uint8(&self, value: u8, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint16LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteuint16levalue-offset): write a little-endian `u16` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16LE")]
    pub fn write_uint16_le(&self, value: u16, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint16BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteuint16bevalue-offset): write a big-endian `u16` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint16BE")]
    pub fn write_uint16_be(&self, value: u16, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint32LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteuint32levalue-offset): write a little-endian `u32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32LE")]
    pub fn write_uint32_le(&self, value: u32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeUint32BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteuint32bevalue-offset): write a big-endian `u32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeUint32BE")]
    pub fn write_uint32_be(&self, value: u32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt8(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteint8value-offset): write a signed byte at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt8")]
    pub fn write_int8(&self, value: i8, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt16LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteint16levalue-offset): write a little-endian `i16` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16LE")]
    pub fn write_int16_le(&self, value: i16, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt16BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteint16bevalue-offset): write a big-endian `i16` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt16BE")]
    pub fn write_int16_be(&self, value: i16, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt32LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteint32levalue-offset): write a little-endian `i32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32LE")]
    pub fn write_int32_le(&self, value: i32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeInt32BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwriteint32bevalue-offset): write a big-endian `i32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeInt32BE")]
    pub fn write_int32_be(&self, value: i32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeFloatLE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritefloatlevalue-offset): write a little-endian `f32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatLE")]
    pub fn write_float_le(&self, value: f32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeFloatBE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritefloatbevalue-offset): write a big-endian `f32` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeFloatBE")]
    pub fn write_float_be(&self, value: f32, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeDoubleLE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritedoublelevalue-offset): write a little-endian `f64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleLE")]
    pub fn write_double_le(&self, value: f64, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeDoubleBE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritedoublebevalue-offset): write a big-endian `f64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeDoubleBE")]
    pub fn write_double_be(&self, value: f64, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigInt64LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritebigint64levalue-offset): write a little-endian `i64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64LE")]
    pub fn write_big_int64_le(&self, value: i64, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigInt64BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritebigint64bevalue-offset): write a big-endian `i64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigInt64BE")]
    pub fn write_big_int64_be(&self, value: i64, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigUint64LE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritebiguint64levalue-offset): write a little-endian `u64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64LE")]
    pub fn write_big_uint64_le(&self, value: u64, offset: Option<u32>) -> u32 {
        unreachable!()
    }

    /// [`buf.writeBigUint64BE(value, offset)`](https://nodejs.org/api/buffer.html#bufwritebiguint64bevalue-offset): write a big-endian `u64` at `offset`, or 0, and give the offset after it.
    #[cfg_attr(rust_js, rust_js::link_name = "writeBigUint64BE")]
    pub fn write_big_uint64_be(&self, value: u64, offset: Option<u32>) -> u32 {
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
    pub r#type: String,
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
}

/// [`buffer.transcode(source, fromEnc, toEnc)`](https://nodejs.org/api/buffer.html#buffertranscodesource-fromenc-toenc):
/// `source`'s text, as `from_enc`, in `to_enc`.
#[cfg_attr(rust_js, rust_js::link_name = "buffer#transcode")]
pub fn transcode(source: &Uint8Array, from_enc: TranscodeEncoding, to_enc: TranscodeEncoding) -> &'static Buffer {
    unreachable!()
}

/// [`buffer.resolveObjectURL(id)`](https://nodejs.org/api/buffer.html#bufferresolveobjecturlid):
/// the `Blob` a `blob:` URL `URL.createObjectURL` made is, or `None`.
#[cfg_attr(rust_js, rust_js::link_name = "buffer#resolveObjectURL")]
pub fn resolve_object_url(id: &str) -> Option<&'static Blob> {
    unreachable!()
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
/// what it searches for, takes: each as it is (ADR 0229); an optional one,
/// `Option<&dyn IntoBufferFill>`, borrowed, `Some(&value)` (ADR 0443).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `string | Uint8Array | number`",
    note = "an optional one, `Option<&dyn IntoBufferFill>`, takes `Some(&value)`"
)]
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
