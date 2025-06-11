//! MAString, a string type designed to minimise memory allocations.
//!
//! This crate provides four types, MAByteString stores arbitrary
//! sequences of bytes, while MAString stores valid UTF-8.
//! MAByteStringBuilder and MAStringbuilder are similar to 
//! MAByteString and MAString but do not allow shared ownership.
//!
//! There are a number of reference-counted string types for rust,
//! However, these commonly require memory allocation when converting
//! From a alloc::String, which means adopting them can actually
//! increase memory allocator calls.
//!
//! This crate attempts to solve that, it is currently at the alpha
//! stage and has only been very lightly tested.
//!
//! A MAString or MABytestring is four pointers in size, and can be in one of
//! five modes, the mode can be checked through the "mode" method.
//! which returns a string representing the current mode and if the string
//! is in a shared ownership mode whether or not it is actually shared.
//! 
//! There are five possible modes.
//! * Short string ("short"): the string data is stored entirely
//!   within the MAString object.
//! * Static string ("static"): the string stores a pointer to a
//!   string with static lifetime
//! * Uniquely owned string ("unique"): the string stores a pointer to a
//!   string that is uniquely owned by the current MAString object.
//! * Reference counted string with inline control block  ("cbinline"):
//!   the string stores a pointer to a reference counted string,
//! * Reference counted string with seperate control block  ("cbowned"):
//!   the string stores a pointer to a reference counted string,
//!
//! A uniquely owned MAString will be converted to one with shared
//! ownership, and a seperate control block by Clone calls. This
//! means repeatedly cloning a string will result in at-most
//! a single memory allocation.
//!
//! MAStrings are represented as a union. The short string variant
//! stores string data and a single byte length. 0x80 is added to
//! the length of a short string to distinguish it from a long 
//! string.
//!
//! The long string variant stores a pointer, length, "capacity"
//! and control block pointer, the "capacity" includes space
//! used to store an inline control block if-any and is set
//! to zero to indicate a static string.
//!
//! The most significant byte of the long string length shares
//! A memory location with the short string length. Since the
//! long length field is always less than isize::max and the 
//! short length field is always greater than or equal to 0x80,
//! this allows short and long strings to be distinguished.  The control
//! block pointer is stored as an atomic pointer, to allow a
//! uniquely owned string to be converted to a shared ownership
//! string by the clone function.
//!
//! The control block is an atomic usize, with the lower
//! bit used to distinguish between seperately owned, and inline
//! control blocks, and the remaining bits used as a reference
//! count.

#![no_std]
#![warn(unsafe_op_in_unsafe_fn)]

mod limitedusize;
mod inner;
mod bytestring;
pub use bytestring::MAByteString;
mod bytestringbuilder;
pub use bytestringbuilder::MAByteStringBuilder;
mod string;
pub use string::MAString;
#[doc(hidden)]
pub use string::chars_utf8len;
#[doc(hidden)]
pub use string::chars_to_bytes;
mod customcow;
pub use customcow::CustomCow;

mod stringbuilder;
pub use stringbuilder::MAStringBuilder;
extern crate alloc;


