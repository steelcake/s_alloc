use std::alloc::AllocError;
use std::io;
use std::ptr::NonNull;

use super::PageAlloc;

pub struct DynamicPageAlloc;

// Safety: moving the struct doesn't invalidate currently allocated pages
unsafe impl PageAlloc for DynamicPageAlloc {
    fn alloc_page(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        assert!(size > 0);

        let alloc_size = size.next_multiple_of(1 << 21); // round up to next multiple of 2MB

        let page = match mmap_wrapper(alloc_size) {
            Ok(page) => page,
            Err(e) => {
                eprintln!("failed to allocate memory with mmap: {}.\naborting.", e);
                std::process::abort();
            }
        };

        // Safety: we know page is valid allocated memory that has more than 0 size.
        // there is nothing indicating this can cause UB in man page of madvise
        unsafe {
            match libc::madvise(
                page.as_ptr().as_mut_ptr() as *mut libc::c_void,
                page.len(),
                libc::MADV_HUGEPAGE,
            ) {
                0 => (),
                -1 => {
                    let errno = *libc::__errno_location();
                    let err = std::io::Error::from_raw_os_error(errno);

                    // TODO: maybe shouldn't abort here :)
                    eprintln!(
                        "failed to call madvise on allocated memory: {}.\naborting",
                        err
                    );
                    std::process::abort();
                }
                x => {
                    // TODO: maybe shouldn't abort here :)
                    eprintln!(
                        "unexpected return value from madvise: {}. Expected 0 or -1\naborting",
                        x
                    );
                    std::process::abort();
                }
            }
        }

        Ok(page)
    }

    unsafe fn dealloc_page(&self, page: NonNull<[u8]>) {
        let ptr = page.as_ptr().as_mut_ptr();
        let size = page.len();

        if let Err(e) = munmap_wrapper(ptr, size) {
            eprintln!("failed to deallocate page with munmap: {}\naborting.", e);
            std::process::abort();
        }
    }
}

fn mmap_wrapper(size: usize) -> io::Result<NonNull<[u8]>> {
    assert!(size > 0);
    // Safety: Call format fits mmap manpage, should be safe
    unsafe {
        match libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_POPULATE,
            -1,
            0,
        ) {
            libc::MAP_FAILED => {
                let errno = *libc::__errno_location();
                let err = std::io::Error::from_raw_os_error(errno);
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("mmap returned error: {}", err),
                ))
            }
            ptr => match NonNull::new(ptr as *mut u8) {
                Some(ptr) => Ok(NonNull::slice_from_raw_parts(ptr, size)),
                None => Err(io::Error::new(
                    io::ErrorKind::Other,
                    "mmap returned null pointer",
                )),
            },
        }
    }
}

unsafe fn munmap_wrapper(ptr: *mut u8, size: usize) -> io::Result<()> {
    match libc::munmap(ptr as *mut libc::c_void, size) {
        0 => Ok(()),
        -1 => {
            let errno = *libc::__errno_location();
            let err = std::io::Error::from_raw_os_error(errno);
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to free memory: {}", err),
            ))
        }
        x => Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "unexpected return value from munmap: {}. Expected 0 or -1",
                x
            ),
        )),
    }
}
