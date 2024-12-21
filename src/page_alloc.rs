use core::alloc::AllocError;
use core::ptr::NonNull;

pub unsafe trait PageAlloc {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError>;
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) -> Result<(), AllocError>;
}

pub struct ThpAlloc;

unsafe impl PageAlloc for ThpAlloc {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) -> Result<(), AllocError> {
        todo!()
    }
}

pub struct HugePage1GBAlloc;

unsafe impl PageAlloc for HugePage1GBAlloc {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) -> Result<(), AllocError> {
        todo!()
    }
}

pub struct HugePage2MBAlloc;

unsafe impl PageAlloc for HugePage2MBAlloc {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) -> Result<(), AllocError> {
        todo!()
    }
}

unsafe impl PageAlloc for std::alloc::Global {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) -> Result<(), AllocError> {
        todo!()
    }
}
