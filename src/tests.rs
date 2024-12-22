use std::alloc::{AllocError, Allocator, Layout};
use std::ptr::NonNull;

use crate::{
    bump_alloc::{self, BumpAlloc},
    local_alloc::{self, LocalAlloc},
};

#[test]
fn test_local_alloc() {
    let alloc = LocalAlloc::new(local_alloc::Config::new(&std::alloc::Global));
    test_allocator_all(alloc);
}

#[test]
fn test_bump_alloc() {
    let alloc = BumpAlloc::new(bump_alloc::Config::new(std::alloc::Global));
    test_allocator_all(alloc);
}

#[test]
fn test_local_bump_alloc() {
    let alloc = LocalAlloc::new(local_alloc::Config::new(&std::alloc::Global));
    let alloc = BumpAlloc::new(bump_alloc::Config::new(&alloc));
    test_allocator_all(alloc);
}

fn test_allocator<Alloc: Allocator>(alloc: Alloc) {
    let alloc = ValidatingAllocator {
        inner: alloc,
    };

}

fn test_allocator_aligned<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_large_alignment<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_aligned_shrink<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_all<Alloc: Allocator>(alloc: Alloc) {
    test_allocator(&alloc);
    test_allocator_aligned(&alloc);
    test_allocator_large_alignment(&alloc);
    test_allocator_aligned_shrink(&alloc);
}

struct ValidatingAllocator<Alloc: Allocator> {
    inner: Alloc,
}

fn check_layout(slice: NonNull<[u8]>, layout: Layout) {
    assert_eq!(slice.as_ptr().as_mut_ptr().align_offset(layout.align()), 0);
    assert!(slice.len() >= layout.size());
}

unsafe impl<Alloc: Allocator> Allocator for ValidatingAllocator<Alloc> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.allocate(layout)?;
        check_layout(x, layout);
        Ok(x)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout);
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.grow(ptr, old_layout, new_layout)?;
        check_layout(x, new_layout);
        Ok(x)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.shrink(ptr, old_layout, new_layout)?;
        check_layout(x, new_layout);
        Ok(x)
    }
}
