use core::sync::atomic::{AtomicBool, Ordering};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

static HEAP_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_heap() {
    if HEAP_INITIALIZED.swap(true, Ordering::SeqCst) {
        panic!("heap already initialized");
    }

    let heap_block = super::layout::heap_block();
    let start = heap_block.base();
    let size = heap_block.size();

    crate::kinfo!(
        "Heap: {:#010x}..{:#010x}, size={}",
        start,
        start + size,
        size
    );

    unsafe {
        ALLOCATOR.lock().init(start as *mut u8, size);
    }
}

pub fn heap_total() -> usize {
    ALLOCATOR.lock().size()
}

pub fn heap_used() -> usize {
    ALLOCATOR.lock().used()
}

pub fn heap_free() -> usize {
    ALLOCATOR.lock().free()
}
