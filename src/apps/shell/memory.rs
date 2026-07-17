use crate::apps::shell::ShellApp;
use crate::println;
use crate::sys::memory::{heap, layout};
use crate::sys::syscall;
use crate::sys::task::Priority;

const MEMORY_PRIO: u8 = 100;
const MEMORY_STACK_SIZE: usize = 256;

extern "C" fn mem_task(_arg: *mut ()) {
    println!("Memory Layout");
    println!("-------------");

    println!(
        "RAM       : {:#010x}..{:#010x} ({} KB)",
        layout::ram_start(),
        layout::ram_end(),
        (layout::ram_end() - layout::ram_start()) / 1024
    );

    println!(".bss end  : {:#010x}", layout::heap_start());

    println!(
        "Heap      : {:#010x}..{:#010x} ({} KB)",
        layout::heap_start(),
        layout::heap_end(),
        layout::heap_size() / 1024
    );

    println!(
        "StackPool : {:#010x}..{:#010x} ({} KB)",
        layout::stack_pool_start(),
        layout::stack_pool_end(),
        layout::stack_pool_size() / 1024
    );

    println!("Reserve   : {} KB", layout::reserve_size() / 1024);

    println!(
        "Gap       : {} KB",
        (layout::stack_pool_start() - layout::heap_end()) / 1024
    );

    println!();

    println!("StackPool");
    println!("---------");
    println!("Total     : {} bytes", syscall::stack_pool_total());
    println!("Used      : {} bytes", syscall::stack_pool_used());
    println!("Free      : {} bytes", syscall::stack_pool_free());

    println!();

    println!("Heap");
    println!("----");
    println!("Total     : {} bytes", heap::heap_total());
    println!("Used      : {} bytes", heap::heap_used());
    println!("Free      : {} bytes", heap::heap_free());
}

pub(super) static MEM_APP: ShellApp = ShellApp::new(
    "mem",
    "Show memory information",
    mem_task,
    MEMORY_STACK_SIZE,
    Priority(MEMORY_PRIO),
);
