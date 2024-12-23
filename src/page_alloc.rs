use core::alloc::{AllocError, Layout};
use core::ptr::NonNull;
use std::alloc::Allocator;

/// # Safety
///
/// moving an implementation of this trait shouldn't invalidate currently allocated pages.
pub unsafe trait PageAlloc {
    /// Returns a pointer aligned to at least 4KB
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError>;
    /// # Safety
    ///
    /// page has to be a currently allocated page from this instance of PageAlloc
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>);
}

unsafe impl PageAlloc for std::alloc::Global {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        let alloc_size = size.next_multiple_of(1 << 12);
        let layout = Layout::from_size_align(alloc_size, 1 << 12).unwrap();
        self.allocate(layout)
    }
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) {
        self.deallocate(
            page.cast::<u8>(),
            Layout::from_size_align(page.len(), 1 << 12).unwrap(),
        );
    }
}

#[cfg_attr(target_os = "linux", path = "./page_alloc/linux.rs")]
mod dynamic_page_alloc;
