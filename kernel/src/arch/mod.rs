#[cfg(not(feature = "cortex-m"))]
compile_error!("No architecture selected");

use minirtos_abi::SysError;

mod interface;

pub(crate) use interface::Arch;

#[cfg(feature = "cortex-m")]
#[path = "arm/cortex_m/mod.rs"]
mod imp;

#[cfg(feature = "cortex-m")]
pub type Context = <imp::CortexM as Arch>::Context;

#[cfg(feature = "cortex-m")]
pub type IrqState = <imp::CortexM as Arch>::IrqState;

#[cfg(feature = "cortex-m")]
pub type ProtectionContext = <imp::CortexM as Arch>::ProtectionContext;

#[cfg(feature = "cortex-m")]
type CurrentArch = imp::CortexM;

pub fn init(core_clock_hz: u32, tick_hz: u32) {
    CurrentArch::init(core_clock_hz, tick_hz);
}

#[inline]
pub fn stack_alignment() -> usize {
    CurrentArch::STACK_ALIGNMENT
}

#[inline]
pub fn memory_alignment() -> usize {
    CurrentArch::MEMORY_ALIGNMENT
}

#[inline]
pub const fn memory_region_count() -> usize {
    CurrentArch::MEMORY_REGION_COUNT
}

pub fn init_context(
    stack_top: *mut u8,
    entry: crate::task::TaskEntry,
    arg: *mut (),
    exit: crate::task::TaskExit,
    privilege: crate::task::Privilege,
) -> Context {
    CurrentArch::init_context(stack_top, entry, arg, exit, privilege)
}

pub fn stack_pointer(context: &Context) -> usize {
    CurrentArch::stack_pointer(context)
}

pub fn set_stack_pointer(context: &mut Context, sp: usize) {
    CurrentArch::set_stack_pointer(context, sp);
}

pub fn prepare_context_switch(context: &Context) {
    CurrentArch::prepare_context_switch(context);
}

pub fn start_first_task() -> ! {
    CurrentArch::start_first_task()
}

pub fn request_context_switch() {
    CurrentArch::request_context_switch();
}

pub fn disable_interrupts() -> IrqState {
    CurrentArch::disable_interrupts()
}

pub fn restore_interrupts(state: IrqState) {
    CurrentArch::restore_interrupts(state);
}

pub fn interrupts_enabled() -> bool {
    CurrentArch::interrupts_enabled()
}

pub fn wait_for_interrupt() {
    CurrentArch::wait_for_interrupt();
}

pub fn init_protection() {
    CurrentArch::init_protection();
}

pub fn apply_protection(context: &ProtectionContext) {
    CurrentArch::apply_protection(context);
}

pub fn clear_protection() {
    CurrentArch::clear_protection();
}

pub fn new_protection_context() -> ProtectionContext {
    CurrentArch::new_protection_context()
}

pub fn add_stack_region(
    context: &mut ProtectionContext,
    base: usize,
    size: usize,
) -> Result<(), SysError> {
    CurrentArch::add_stack_region(context, base, size)
}

pub fn add_text_region(
    context: &mut ProtectionContext,
    base: usize,
    size: usize,
) -> Result<(), SysError> {
    CurrentArch::add_text_region(context, base, size)
}

pub fn add_device_region(
    context: &mut ProtectionContext,
    base: usize,
    size: usize,
) -> Result<(), SysError> {
    CurrentArch::add_device_region(context, base, size)
}

pub fn add_rw_region(
    context: &mut ProtectionContext,
    base: usize,
    size: usize,
) -> Result<(), SysError> {
    CurrentArch::add_rw_region(context, base, size)
}

pub fn remove_region(
    context: &mut ProtectionContext,
    base: usize,
    size: usize,
) -> Result<(), SysError> {
    CurrentArch::remove_region(context, base, size)
}

pub fn is_privileged() -> bool {
    CurrentArch::is_privileged()
}

pub fn syscall<const ID: u8>(args: &[u32]) -> u32 {
    CurrentArch::syscall::<ID>(args)
}

pub fn syscall_u64<const ID: u8>(args: &[u32]) -> u64 {
    CurrentArch::syscall_u64::<ID>(args)
}

pub fn syscall_noreturn<const ID: u8>(args: &[u32]) -> ! {
    CurrentArch::syscall_noreturn::<ID>(args)
}

pub fn current_thread_sp() -> usize {
    CurrentArch::current_thread_sp()
}

pub fn exception_number() -> usize {
    CurrentArch::exception_number()
}
