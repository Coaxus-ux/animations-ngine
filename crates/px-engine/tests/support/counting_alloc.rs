//! A global allocator that counts heap activity, for tests certifying that rendering
//! does not allocate (`docs/engineering/RUST-STANDARDS.md` §13). Shared by every crate's
//! `no_alloc` test through `#[path]`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every method forwards to `System` with the caller's arguments unchanged; the
// counters are atomics and do not touch the allocated memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as `GlobalAlloc::alloc`, forwarded verbatim.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as `GlobalAlloc::alloc_zeroed`, forwarded verbatim.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `ptr` and `layout` come from a matching `alloc` on `System`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: same contract as `GlobalAlloc::realloc`, forwarded verbatim.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Runs `work` and fails if it allocated or freed heap memory.
pub fn assert_no_heap_activity(work: impl FnOnce()) {
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let deallocations = DEALLOCATIONS.load(Ordering::Relaxed);
    work();
    assert_eq!(
        ALLOCATIONS.load(Ordering::Relaxed),
        allocations,
        "allocated"
    );
    assert_eq!(
        DEALLOCATIONS.load(Ordering::Relaxed),
        deallocations,
        "freed memory"
    );
}
