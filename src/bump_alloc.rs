use core::alloc::{AllocError, Allocator, Layout};
use std::cell::RefCell;
use std::ops::DerefMut;
use std::ptr::NonNull;

use crate::util::align_offset;

pub struct Config<Alloc: Allocator> {
    base_alloc: Alloc,
    error_after: usize,
    min_alloc_size: usize,
}

impl<Alloc: Allocator> Config<Alloc> {
    pub fn new(base_alloc: Alloc) -> Self {
        Self {
            base_alloc,
            error_after: usize::MAX,
            min_alloc_size: 1 << 24, // 16 MB
        }
    }

    pub fn error_after(&mut self, error_after: usize) -> &mut Self {
        self.error_after = error_after;
        self
    }

    pub fn min_alloc_size(&mut self, min_alloc_size: usize) -> &mut Self {
        self.min_alloc_size = min_alloc_size;
        self
    }
}

// Use this to avoid constructing aliasing pointers
type Ptr = usize;

#[derive(Clone, Copy)]
struct Slice {
    ptr: Ptr,
    len: usize,
}

struct InnerBumpAlloc<Alloc: Allocator> {
    base_alloc: Alloc,
    error_after: usize,
    min_alloc_size: usize,
    total_alloc_size: usize,
    allocations: Vec<Slice>,
    current_alloc: Slice,
}

pub struct BumpAlloc<Alloc: Allocator> {
    inner: RefCell<InnerBumpAlloc<Alloc>>,
}

impl<Alloc: Allocator> BumpAlloc<Alloc> {
    pub fn new(config: Config<Alloc>) -> Self {
        Self {
            inner: RefCell::new(InnerBumpAlloc {
                base_alloc: config.base_alloc,
                error_after: config.error_after,
                min_alloc_size: config.min_alloc_size,
                total_alloc_size: 0,
                allocations: Vec::new(),
                current_alloc: Slice {
                    // this errors if we don't do the cast, so suppress clippy warning
                    #[allow(clippy::unnecessary_cast)]
                    ptr: NonNull::dangling().as_ptr() as *mut u8 as usize,
                    len: 0,
                },
            }),
        }
    }
}

// Safety: Allocations don't get invalidated when BumpAlloc is moved.
unsafe impl<Alloc: Allocator> Allocator for BumpAlloc<Alloc> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let mut this = self.inner.borrow_mut();
        let this = this.deref_mut();

        if this.error_after <= this.total_alloc_size {
            return Err(AllocError);
        }

        // Alignment above 4KB is not allwed
        if layout.align() > 1 << 12 {
            return Err(AllocError);
        }

        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }

        let align_offs = align_offset(this.current_alloc.ptr, layout.align());

        if this.current_alloc.len >= align_offs + layout.size() {
            let ptr = NonNull::new(this.current_alloc.ptr as *mut u8).unwrap();

            this.current_alloc.ptr += align_offs + layout.size();
            this.current_alloc.len -= align_offs + layout.size();

            return Ok(NonNull::slice_from_raw_parts(ptr, layout.size()));
        }

        let alloc_size = layout.size().next_multiple_of(this.min_alloc_size);
        let alloc_layout = Layout::from_size_align(alloc_size, 1 << 12).unwrap();

        let new_alloc = this.base_alloc.allocate(alloc_layout)?;
        let new_alloc = Slice {
            ptr: new_alloc.as_ptr().as_mut_ptr() as usize,
            len: new_alloc.len(),
        };

        this.allocations.push(new_alloc);

        this.current_alloc = Slice {
            ptr: new_alloc.ptr + layout.size(),
            len: new_alloc.len - layout.size(),
        };

        let ptr = NonNull::new(new_alloc.ptr as *mut u8).unwrap();

        Ok(NonNull::slice_from_raw_parts(ptr, layout.size()))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let mut this = self.inner.borrow_mut();
        let this = this.deref_mut();

        // Increasing the alignment is not supported
        // Decrasing doesn't matter since we don't move the pointer around
        if old_layout.align() < new_layout.align() {
            return Err(AllocError);
        }

        if new_layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }

        if new_layout.size() <= old_layout.size() {
            return Err(AllocError);
        }

        let start_addr = (ptr.as_ptr() as usize) + old_layout.size();

        let size_diff = new_layout.size() - old_layout.size();
        if this.current_alloc.ptr == start_addr && this.current_alloc.len >= size_diff {
            this.current_alloc.ptr += size_diff;
            this.current_alloc.len -= size_diff;

            return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
        }

        Err(AllocError)
    }
}
