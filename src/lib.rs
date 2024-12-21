#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![allow(clippy::comparison_chain)]

pub mod bump_alloc;
pub mod local_alloc;
pub mod page_alloc;
mod util;
