use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::sys::synchronization::{IrqSafeNullLock, interface::Mutex};

use super::{scheduler, synchronization::critical_section};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,  // waiting on mutex/semaphore
    Sleeping, // waiting on timer
    Suspended,
    Terminated,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(pub usize);

static NEXT_TASK_ID: AtomicU32 = AtomicU32::new(0);

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Priority(pub u8);

#[allow(dead_code)]
pub(super) struct TaskControlBlock {
    /// Saved stack pointer — MUST be first field (asm relies on offset 0)
    pub sp: *mut u32,

    /// Task stack storage.
    pub stack: &'static mut [u32],
    pub stack_bottom: *mut u8,
    pub stack_size: usize,

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

    /// time slice for the task
    pub time_slice: u32,
    /// remaining time slice for the task
    pub remaining_slice: u32,
}

pub type TaskEntry = extern "C" fn(*mut ()) -> !;

/// Called if a task entry function ever returns.
extern "C" fn task_exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

const STACK_MAGIC: u32 = 0xdead_beef;
const STACK_GUARD_WORDS: usize = 16;
const DEFAULT_TIME_SLICE: u32 = 5;

/// Stack container
pub(super) struct TaskStack<const N: usize>(UnsafeCell<[u32; N]>);

unsafe impl<const N: usize> Sync for TaskStack<N> {}

impl<const N: usize> TaskStack<N> {
    pub const fn new() -> Self {
        Self(UnsafeCell::new([0; N]))
    }

    pub fn get(&self) -> &'static mut [u32; N] {
        unsafe { &mut *self.0.get() }
    }
}

/// Task control block
#[allow(dead_code)]
impl TaskControlBlock {
    pub fn new(
        entry: TaskEntry,
        arg: *mut (),
        stack: &'static mut [u32],
        priority: Priority,
        name: &'static str,
    ) -> Self {
        let id = TaskId(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed) as usize);

        // Fill stack with magic pattern for watermark / overflow detection
        for word in stack.iter_mut() {
            *word = STACK_MAGIC;
        }

        // Take metadata before moving `stack` into the TCB.
        let stack_bottom = stack.as_mut_ptr() as *mut u8;
        let stack_size = stack.len() * core::mem::size_of::<u32>();

        let mut tcb = Self {
            sp: core::ptr::null_mut(),
            stack,
            stack_bottom,
            stack_size,
            id,
            state: TaskState::Ready,
            priority,
            base_priority: priority,
            wake_tick: 0,
            name,
            entry,
            arg,
            time_slice: DEFAULT_TIME_SLICE,
            remaining_slice: DEFAULT_TIME_SLICE,
        };

        tcb.sp = tcb.init_stack(entry, arg);

        tcb
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

    pub fn with_time_slice(mut self, time_slice: u32) -> Self {
        self.time_slice = time_slice;
        self.remaining_slice = time_slice;
        self
    }

    pub fn stack_total_bytes(&self) -> usize {
        self.stack.len() * core::mem::size_of::<u32>()
    }

    pub fn stack_used_bytes(&self) -> usize {
        let unused_words = self
            .stack
            .iter()
            .take_while(|&&word| word == STACK_MAGIC)
            .count();

        (self.stack.len() - unused_words) * core::mem::size_of::<u32>()
    }

    pub fn stack_free_bytes(&self) -> usize {
        self.stack_total_bytes() - self.stack_used_bytes()
    }

    pub fn stack_guard_ok(&self) -> bool {
        self.stack
            .iter()
            .take(STACK_GUARD_WORDS)
            .all(|&word| word == STACK_MAGIC)
    }

    pub fn stack_sp_in_range(&self) -> bool {
        let sp = self.sp as usize;
        let bottom = self.stack.as_ptr() as usize;
        let top = unsafe { self.stack.as_ptr().add(self.stack.len()) } as usize;

        sp >= bottom && sp <= top
    }

    pub fn check_stack_guard(&self) {
        if !self.stack_guard_ok() {
            defmt::panic!(
                "stack overflow: task={} used={} total={}",
                self.name,
                self.stack_used_bytes(),
                self.stack_total_bytes(),
            );
        }
    }
}

unsafe impl Send for TaskControlBlock {}

// Reserve some RAM at the top of memory for the MSP (Main Stack Pointer),
// interrupt handlers, exceptions, panic handling, and early runtime startup.
//
// Cortex-M uses MSP by default after reset, and exceptions/interrupts
// continue using MSP even when tasks run with PSP.
//
// Task stacks are allocated below this reserved region to avoid
// corrupting the kernel/interrupt stack.
const KERNEL_STACK_RESERVE: usize = 16 * 1024;

pub struct StackPool<const SIZE: usize> {
    bottom: usize,
    current: usize,
    initialized: bool,
}

impl<const SIZE: usize> StackPool<SIZE> {
    pub const fn empty() -> Self {
        Self {
            bottom: 0,
            current: 0,
            initialized: false,
        }
    }

    fn init_once(&mut self) {
        if !self.initialized {
            let ram_end = crate::sys::memory::ram_end();

            let end = ram_end - KERNEL_STACK_RESERVE;

            self.bottom = end - SIZE;
            self.current = end;
            self.initialized = true;

            defmt::info!("Stack pool: {:#010x}..{:#010x}", self.bottom, self.current);
        }
    }

    pub fn alloc_words(&mut self, words: usize) -> Result<&'static mut [u32], super::SysError> {
        self.init_once();

        let words = (words + 1) & !1;
        let bytes = words * core::mem::size_of::<u32>();

        let new_current = (self.current - bytes) & !7;

        if new_current < self.bottom {
            return Err(super::SysError::NoMemory);
        }

        self.current = new_current;

        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                new_current as *mut u32,
                words,
            ))
        }
    }
}

const STACK_POOL_SIZE: usize = 64 * 1024;

static STACK_POOL: IrqSafeNullLock<StackPool<STACK_POOL_SIZE>> =
    IrqSafeNullLock::new(StackPool::empty());

/// Interface for apps
pub struct Task<const STACK_WORDS: usize> {
    entry: TaskEntry,
    arg: *mut (),
    priority: Priority,
    name: &'static str,
    task_id: Option<TaskId>,
}

#[allow(dead_code)]
impl<const STACK_WORDS: usize> Task<STACK_WORDS> {
    pub const fn new(entry: TaskEntry) -> Self {
        Self {
            entry,
            arg: core::ptr::null_mut(),
            priority: Priority(128),
            name: "",
            task_id: None,
        }
    }

    pub fn arg(mut self, arg: *mut ()) -> Self {
        self.arg = arg;
        self
    }

    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn run(&mut self) -> Result<TaskId, super::SysError> {
        if self.task_id.is_some() {
            return Err(super::SysError::Busy);
        }

        let stack = STACK_POOL.lock(|pool| pool.alloc_words(STACK_WORDS))?;

        let task_id = critical_section(|cs| {
            scheduler::scheduler().add_task(
                cs,
                self.entry,
                self.arg,
                stack,
                self.priority,
                self.name,
            )
        })?;

        self.task_id = Some(task_id);
        Ok(task_id)
    }
}
