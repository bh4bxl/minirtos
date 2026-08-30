pub mod context;
mod exception;
mod interrupt;
mod mpu;
mod timer;

use minirtos_abi::SysError;

use crate::task::{Privilege, TaskEntry, TaskExit};

pub struct CortexM;

impl super::Arch for CortexM {
    type Context = context::Context;
    type IrqState = interrupt::IrqState;
    type ProtectionContext = mpu::ProtectionContext;

    fn init(core_clock_hz: u32, tick_hz: u32) {
        let cp = cortex_m::Peripherals::take().unwrap();

        exception::init(cp.SCB);

        timer::init_timer(cp.SYST, core_clock_hz, tick_hz);
    }

    #[inline]
    fn stack_alignment() -> usize {
        8
    }

    fn init_context(
        stack_top: *mut u8,
        entry: TaskEntry,
        arg: *mut (),
        exit: TaskExit,
        privilege: Privilege,
    ) -> Self::Context {
        context::init(stack_top, entry, arg, exit, privilege)
    }

    fn stack_pointer(context: &Self::Context) -> usize {
        context::stack_pointer(context)
    }

    fn start_first_task() -> ! {
        exception::svc::start_first_task()
    }

    fn request_context_switch() {
        exception::pend_pendsv();
    }

    fn set_stack_pointer(context: &mut Self::Context, sp: usize) {
        context::set_stack_pointer(context, sp);
    }

    fn prepare_context_switch(context: &Self::Context) {
        context::prepare_switch(context);
    }

    fn disable_interrupts() -> Self::IrqState {
        interrupt::disable()
    }

    fn restore_interrupts(state: Self::IrqState) {
        interrupt::restore(state);
    }

    fn interrupts_enabled() -> bool {
        interrupt::enabled()
    }

    fn wait_for_interrupt() {
        cortex_m::asm::wfi();
    }

    fn init_protection() {
        mpu::init();
    }

    fn apply_protection(context: &Self::ProtectionContext) {
        mpu::apply(context);
    }

    fn clear_protection() {
        mpu::clear();
    }

    fn new_protection_context() -> Self::ProtectionContext {
        mpu::ProtectionContext::new()
    }

    fn add_stack_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError> {
        context
            .add_region(mpu::Region::read_write(base, size))
            .map_err(|_| SysError::NoResource)
    }

    fn add_text_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError> {
        context
            .add_region(mpu::Region::code(base, size))
            .map_err(|_| SysError::NoResource)
    }

    fn add_device_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError> {
        context
            .add_region(mpu::Region::device(base, size))
            .map_err(|_| SysError::NoResource)
    }

    fn add_rw_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError> {
        context
            .add_region(mpu::Region::read_write(base, size))
            .map_err(|_| SysError::NoResource)
    }

    fn remove_region(
        context: &mut Self::ProtectionContext,
        base: usize,
        size: usize,
    ) -> Result<(), minirtos_abi::SysError> {
        context
            .remove_region(base, size)
            .map_err(|_| SysError::NoResource)
    }

    fn is_privileged() -> bool {
        context::is_privileged()
    }

    fn syscall<const ID: u8>(args: &[u32]) -> u32 {
        assert!(args.len() <= 4);

        let mut r0 = args.first().copied().unwrap_or(0);
        let r1 = args.get(1).copied().unwrap_or(0);
        let r2 = args.get(2).copied().unwrap_or(0);
        let r3 = args.get(3).copied().unwrap_or(0);

        unsafe {
            core::arch::asm!(
                "svc {svc}",
                svc = const ID,
                inlateout("r0") r0,
                in("r1") r1,
                in("r2") r2,
                in("r3") r3,
                // options(nostack),
            );
        }

        r0
    }

    fn syscall_u64<const ID: u8>(args: &[u32]) -> u64 {
        assert!(args.len() <= 4);

        let mut r0 = args.get(0).copied().unwrap_or(0);
        let mut r1 = args.get(1).copied().unwrap_or(0);
        let r2 = args.get(2).copied().unwrap_or(0);
        let r3 = args.get(3).copied().unwrap_or(0);

        unsafe {
            core::arch::asm!(
                "svc {svc}",
                svc = const ID,
                inlateout("r0") r0,
                inlateout("r1") r1,
                in("r2") r2,
                in("r3") r3,
                // options(nostack),
            );
        }

        ((r1 as u64) << 32) | r0 as u64
    }

    fn syscall_noreturn<const ID: u8>(args: &[u32]) -> ! {
        assert!(args.len() <= 4);

        let r0 = args.get(0).copied().unwrap_or(0);
        let r1 = args.get(1).copied().unwrap_or(0);
        let r2 = args.get(2).copied().unwrap_or(0);
        let r3 = args.get(3).copied().unwrap_or(0);

        unsafe {
            core::arch::asm!(
                "svc {svc}",
                svc = const ID,
                in("r0") r0,
                in("r1") r1,
                in("r2") r2,
                in("r3") r3,
                options(noreturn),
            );
        }
    }

    fn current_thread_sp() -> usize {
        context::current_psp()
    }

    fn exception_number() -> usize {
        exception::exception_number()
    }
}
