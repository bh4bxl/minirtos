#[unsafe(export_name = "SysTick")]
pub extern "C" fn systick_handler() {
    crate::timer::tick();
}
