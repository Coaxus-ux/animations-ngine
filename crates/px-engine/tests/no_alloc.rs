//! Certifies that `render_frame` performs no heap activity after setup
//! (`docs/engineering/RUST-STANDARDS.md` §13). Run alone: `--test-threads=1`.
#![allow(
    unsafe_code,
    reason = "test-only counting allocator, see DECISIONS.md D-002"
)]
#![allow(
    clippy::unwrap_used,
    reason = "tests should fail loudly on unexpected errors"
)]

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{expr, kit, rect, scene};
use px_core::{CanvasMut, Frame};
use px_engine::{Bounds, Geometry, Param, Segment, Shape, render_frame};

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

#[test]
fn render_frame_performs_no_heap_activity() {
    let kit = kit();
    let ellipse = Bounds {
        x: expr("10 + sin(t * tau) * 8"),
        y: Param::Fixed(4.0),
        width: Param::Fixed(9.0),
        height: expr("6 + f % 3"),
    };
    let line = Segment {
        x1: Param::Fixed(0.0),
        y1: Param::Fixed(0.0),
        x2: expr("max(t * 20, 3)"),
        y2: Param::Fixed(31.0),
    };
    let shapes = vec![
        rect(expr("floor(t * 4)"), Param::Fixed(1.0), 5.0, 5.0, kit.ink),
        Shape {
            geometry: Geometry::Ellipse(ellipse),
            paint: kit.ink,
        },
        Shape {
            geometry: Geometry::Line(line),
            paint: kit.blank,
        },
    ];
    let scene = scene(&kit, (32, 32), 24, shapes);
    let mut cells = vec![kit.blank; scene.size().cell_count()];

    let allocations = ALLOCATIONS.load(Ordering::Relaxed);
    let deallocations = DEALLOCATIONS.load(Ordering::Relaxed);
    for frame in 0..48 {
        let mut canvas = CanvasMut::new(scene.size(), &mut cells).unwrap();
        render_frame(&scene, Frame::new(frame), &mut canvas).unwrap();
    }
    assert_eq!(
        ALLOCATIONS.load(Ordering::Relaxed),
        allocations,
        "render_frame allocated"
    );
    assert_eq!(
        DEALLOCATIONS.load(Ordering::Relaxed),
        deallocations,
        "render_frame freed memory"
    );
}
