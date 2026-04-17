#![no_std]
#![no_main]

use cortex_m::asm;
use defmt_rtt as _;
use panic_probe as _;

use crate::{
    bsp::board_init,
    sys::{cpu::start_first_task, scheduler::set_current_task, task::init_task_stack},
};
use rp235x_hal as hal;

mod bsp;
mod drivers;
mod sys;

// use sys::cpu::start_first_task_with_stack;
use sys::task::TaskControlBlock;

const STACK_WORDS: usize = 256;

#[allow(dead_code)]
#[repr(align(8))]
struct TaskStack([u32; STACK_WORDS]);

static mut TASK1_STACK: TaskStack = TaskStack([0; STACK_WORDS]);
static mut TASK1_TCB: TaskControlBlock = TaskControlBlock::new();

fn task1_entry() -> ! {
    let mut cnt = 0u32;
    loop {
        cnt += 1;
        defmt::info!("task1 running {}", cnt);

        for _ in 0..20_000_000 {
            asm::nop();
        }
    }
}

#[hal::entry]
fn main() -> ! {
    defmt::info!("MINI RTOS");

    match board_init() {
        Err(e) => defmt::error!("Error: {}", e),
        Ok(()) => defmt::info!("Board {} initialized.", sys::board::board().board_name()),
    }

    println!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    unsafe {
        let stack_ptr = core::ptr::addr_of_mut!(TASK1_STACK) as *mut u32;
        let tcb_ptr = core::ptr::addr_of_mut!(TASK1_TCB);

        (*tcb_ptr).sp = init_task_stack(stack_ptr, STACK_WORDS, task1_entry);
        set_current_task(tcb_ptr);

        start_first_task();
    }
}
