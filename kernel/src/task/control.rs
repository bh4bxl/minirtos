use core::sync::atomic::{AtomicU32, Ordering};

use alloc::vec::Vec;
use minirtos_abi::SysError;

use crate::{
    MemoryAccess, MemoryRegion, arch,
    ipc::PendingIpc,
    memory::{self, StackRegion},
    sys,
};

use super::{Priority, Privilege, TaskEntry, TaskId, TaskState};

static NEXT_TASK_ID: AtomicU32 = AtomicU32::new(0);

const STACK_MAGIC: u32 = 0xDEAD_BEEF;
const STACK_GUARD_WORDS: usize = 4;
const DEFAULT_TIME_SLICE: u32 = 5;

pub(crate) struct TaskControl {
    /*
     * Architecture-specific execution context.
     *
     * Cortex-M:
     *   PSP + CONTROL
     *
     * RISC-V:
     *   SP + architecture-specific context state
     */
    pub context: arch::Context,

    /// Architecture-specific memory protection context.
    pub protection: arch::ProtectionContext,

    /// Task stack storage.
    pub stack: StackRegion,

    pub id: TaskId,
    pub state: TaskState,

    pub priority: Priority,

    /// Original priority used by priority inheritance.
    pub base_priority: Priority,

    /// Tick count when this task should wake.
    pub wake_tick: u64,

    /// Human-readable name for debugging.
    pub name: &'static str,

    pub entry: TaskEntry,
    pub arg: *mut (),

    pub time_slice: u32,
    pub remaining_slice: u32,

    pub owned_mutex_count: usize,

    /// Task waiting for this task to terminate.
    pub waiter: Option<TaskId>,

    pub privilege: Privilege,

    pub pending_ipc: PendingIpc,
}

extern "C" fn task_return_trampoline() -> ! {
    sys::exit()
}

impl TaskControl {
    pub fn new(
        entry: TaskEntry,
        arg: *mut (),
        mut stack: StackRegion,
        priority: Priority,
        privilege: Privilege,
        name: &'static str,
        regions: Vec<MemoryRegion>,
    ) -> Result<Self, SysError> {
        let id = TaskId::from_raw(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed) as usize);

        // Fill stack with a known pattern for watermark and
        // overflow detection.
        stack.fill_u32(STACK_MAGIC);

        let stack_top = unsafe { stack.as_mut_ptr().add(stack.size()) } as *mut u8;

        let context = arch::init_context(stack_top, entry, arg, task_return_trampoline, privilege);

        let mut protection = arch::ProtectionContext::new();

        if privilege == Privilege::Unprivileged {
            arch::add_stack_region(&mut protection, stack.start(), stack.size())?;

            let code_block = memory::flash_block();
            arch::add_text_region(&mut protection, code_block.base(), code_block.size())?;

            // Temporary: defmt-rtt uses rp235x-hal critical-section,
            // which accesses SIO CPUID/spinlocks.
            // Remove once user-space logging goes through a syscall/service.
            let ram_block = memory::ram_block();
            arch::add_rw_region(&mut protection, ram_block.base(), 0x5000)?;
            arch::add_device_region(&mut protection, 0xd000_0000, 0x1000)?;

            for region in regions {
                let (base, size) = (region.mem_block().base(), region.mem_block().size());
                match region.access() {
                    MemoryAccess::ReadOnly => {}
                    MemoryAccess::ReadWrite => arch::add_rw_region(&mut protection, base, size)?,
                    MemoryAccess::ExecuteRead => {}
                    MemoryAccess::ExecuteReadWrite => {}
                    MemoryAccess::DeviceReadOnly => {}
                    MemoryAccess::DeviceReadWrite => {
                        arch::add_device_region(&mut protection, base, size)?;
                    }
                }
            }
        }

        Ok(Self {
            context,
            protection,
            stack,

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

            owned_mutex_count: 0,

            waiter: None,

            privilege,

            pending_ipc: PendingIpc::None,
        })
    }

    pub fn with_time_slice(mut self, time_slice: u32) -> Self {
        self.time_slice = time_slice;
        self.remaining_slice = time_slice;
        self
    }

    pub fn into_stack(self) -> StackRegion {
        self.stack
    }

    #[inline]
    pub fn stack_pointer(&self) -> usize {
        arch::stack_pointer(&self.context)
    }

    pub fn stack_total_bytes(&self) -> usize {
        self.stack.size()
    }

    pub fn set_syscall_result(&mut self, result: i32) {
        self.context.set_syscall_result(result);
    }

    pub fn stack_used_bytes(&self) -> usize {
        let unused_words = self.stack.count_prefix_u32(STACK_MAGIC);

        self.stack.size() - unused_words * core::mem::size_of::<u32>()
    }

    pub fn stack_free_bytes(&self) -> usize {
        self.stack_total_bytes() - self.stack_used_bytes()
    }

    pub fn stack_guard_ok(&self) -> bool {
        self.stack.count_prefix_u32(STACK_MAGIC) >= STACK_GUARD_WORDS
    }

    pub fn stack_sp_in_range(&self) -> bool {
        let sp = self.stack_pointer();

        // SP may equal stack.end() when the stack is completely unused.
        sp >= self.stack.start() && sp < self.stack.end()
    }

    pub fn check_stack_guard(&self) {
        if !self.stack_guard_ok() {
            crate::kerror!(
                "stack overflow: task={} used={} total={}",
                self.name,
                self.stack_used_bytes(),
                self.stack_total_bytes(),
            );

            panic!("task stack overflow");
        }
    }

    #[inline]
    pub fn privilege(&self) -> Privilege {
        self.privilege
    }
}

unsafe impl Send for TaskControl {}
