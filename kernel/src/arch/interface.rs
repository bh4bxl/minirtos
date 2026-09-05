use minirtos_abi::SysError;

use crate::task::{Privilege, TaskEntry, TaskExit};

/// Architecture abstraction used by the kernel.
///
/// The kernel must not depend directly on Cortex-M, RISC-V, or any
/// architecture-specific register layout.
pub(crate) trait Arch {
    const MEMORY_REGION_COUNT: usize;
    const MEMORY_ALIGNMENT: usize;
    const STACK_ALIGNMENT: usize;

    /// Architecture-specific saved thread context.
    type Context;

    /// Architecture-specific interrupt state used when temporarily
    /// disabling/restoring interrupts.
    type IrqState: Copy;

    /// Architecture-specific MPU / protection context.
    ///
    /// Use `()` initially if memory protection is not implemented yet.
    type ProtectionContext;

    /// Perform architecture-level initialization required by the kernel.
    ///
    /// Board/platform initialization such as clocks, UART, GPIO, etc.
    /// must already be completed before this is called.
    fn init(core_clock_hz: u32, tick_hz: u32);

    // ---------------------------------------------------------------------
    // Thread context
    // ---------------------------------------------------------------------

    /// Build the initial context for a newly created thread.
    ///
    /// `stack_top` points to the top of the allocated thread stack.
    fn init_context(
        stack_top: *mut u8,
        entry: TaskEntry,
        arg: *mut (),
        exit: TaskExit,
        privilege: Privilege,
    ) -> Self::Context;

    /// Return the current stack pointer stored in a saved context.
    ///
    /// Mainly useful for stack accounting/debugging.
    fn stack_pointer(context: &Self::Context) -> usize;

    /// Start execution of the first scheduled thread.
    ///
    /// This function never returns.
    fn start_first_task() -> !;

    /// Request a context switch.
    ///
    /// On Cortex-M this normally pends PendSV.
    fn request_context_switch();

    fn set_stack_pointer(context: &mut Self::Context, sp: usize);

    fn prepare_context_switch(context: &Self::Context);

    // ---------------------------------------------------------------------
    // Interrupt control
    // ---------------------------------------------------------------------

    /// Disable interrupts and return the previous interrupt state.
    ///
    /// The returned state must later be passed to `restore_interrupts`.
    fn disable_interrupts() -> Self::IrqState;

    /// Restore a previously saved interrupt state.
    fn restore_interrupts(state: Self::IrqState);

    /// Return whether normal interrupts are currently enabled.
    fn interrupts_enabled() -> bool;

    /// Wait for an interrupt / enter the architecture's idle state.
    fn wait_for_interrupt();

    // ---------------------------------------------------------------------
    // Protection / privilege
    // ---------------------------------------------------------------------

    fn init_protection();

    /// Configure architecture-specific memory protection for a thread/process.
    fn apply_protection(context: &Self::ProtectionContext);

    /// Disable/reset user memory protection.
    fn clear_protection();

    fn new_protection_context() -> Self::ProtectionContext;

    fn add_stack_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), SysError>;

    fn add_text_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), SysError>;

    fn add_device_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), SysError>;

    fn add_rw_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError>;

    fn remove_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError>;

    /// Return whether CPU execution is currently privileged.
    fn is_privileged() -> bool;

    // ---------------------------------------------------------------------
    // Syscalls
    // ---------------------------------------------------------------------

    fn syscall<const ID: u8>(args: &[u32]) -> u32;

    fn syscall_u64<const ID: u8>(args: &[u32]) -> u64;

    fn syscall_noreturn<const ID: u8>(args: &[u32]) -> !;

    // ---------------------------------------------------------------------
    // Debug helpers
    // ---------------------------------------------------------------------

    /// Return the current processor stack pointer used by thread mode.
    fn current_thread_sp() -> usize;

    /// Return the current exception/interrupt nesting state if available.
    ///
    /// Zero means normal thread context.
    fn exception_number() -> usize;
}
