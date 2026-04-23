use core::sync::atomic::{AtomicU32, Ordering};

pub const STACK_SIZE: usize = 2048;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Sleeping,
    Suspended,
    Terminated,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(pub u8);

static NEXT_TASK_ID: AtomicU32 = AtomicU32::new(0);

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Priority(pub u8);

#[allow(dead_code)]
pub struct TaskControlBlock {
    /// Saved stack pointer — MUST be first field (asm relies on offset 0)
    pub sp: *mut u32,

    pub id: TaskId,
    pub state: TaskState,
    pub priority: Priority,
    /// For priority inheritance
    pub base_priority: Priority,

    /// Tick count when this task should wake (Sleeping state)
    pub wake_tick: u64,

    /// Human-readable name for debugging
    pub name: &'static str,

    /// Task entry function
    pub entry: TaskEntry,
    /// Task argument
    pub arg: *mut (),

    /// Owned stack allocation
    pub stack: [u32; STACK_SIZE / core::mem::size_of::<u32>()],
}

pub type TaskEntry = extern "C" fn(*mut ()) -> !;

/// Called if a task entry function ever returns.
extern "C" fn task_exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

impl TaskControlBlock {
    pub fn new(entry: TaskEntry, arg: *mut (), priority: Priority, name: &'static str) -> Self {
        let id = TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed) as u8);
        Self {
            sp: core::ptr::null_mut(),
            id,
            state: TaskState::Ready,
            priority,
            base_priority: priority,
            wake_tick: 0,
            name,
            entry,
            arg,
            stack: [0; STACK_SIZE / core::mem::size_of::<u32>()],
        }
    }

    /// Initialize the initial stack frame for a Cortex-M task.
    ///
    /// Stack layout after initialization:
    /// - software-saved registers: r4-r11
    /// - hardware-stacked registers:
    ///   r0, r1, r2, r3, r12, lr, pc, xpsr
    ///
    /// Returns the initial SP value to restore this task.
    pub fn init_stack(&mut self, entry: TaskEntry, arg: *mut ()) -> *mut u32 {
        let stack_top = unsafe { self.stack.as_mut_ptr().add(self.stack.len()) };

        // Cortex-M stacks grow downward; align to 8 bytes.
        let mut sp = (stack_top as usize & !0x7) as *mut u32;

        unsafe {
            // Hardware-stacked exception frame, in reverse

            // xPSR, Thumb bit set
            sp = sp.sub(1);
            *sp = 0x0100_0000;

            // PC = task entry
            sp = sp.sub(1);
            *sp = entry as usize as u32;

            // LR = called if task returns
            sp = sp.sub(1);
            *sp = task_exit as *const () as usize as u32;

            // R12, R3, R2, R1
            sp = sp.sub(1);
            *sp = 0;
            sp = sp.sub(1);
            *sp = 0;
            sp = sp.sub(1);
            *sp = 0;
            sp = sp.sub(1);
            *sp = 0;

            // R0 = argument
            sp = sp.sub(1);
            *sp = arg as u32;

            // Software-saved context (PendSV pops these)

            // R4-R11
            for _ in 0..8 {
                sp = sp.sub(1);
                *sp = 0;
            }
        }

        sp
    }
}
