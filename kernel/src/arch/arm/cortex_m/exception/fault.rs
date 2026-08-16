#[unsafe(no_mangle)]
static mut HF_LR: u32 = 0;

#[unsafe(no_mangle)]
static mut HF_MSP: u32 = 0;

#[unsafe(no_mangle)]
static mut HF_PSP: u32 = 0;

#[unsafe(no_mangle)]
static mut HF_FRAME: u32 = 0;

#[unsafe(export_name = "HardFault")]
#[unsafe(naked)]
pub unsafe extern "C" fn hardfault_handler() -> ! {
    core::arch::naked_asm!(
        "
        ldr r2, ={hf_lr}
        str lr, [r2]

        mrs r1, msp
        ldr r2, ={hf_msp}
        str r1, [r2]

        mrs r1, psp
        ldr r2, ={hf_psp}
        str r1, [r2]

        tst lr, #4
        ite eq
        mrseq r0, msp
        mrsne r0, psp

        ldr r2, ={hf_frame}
        str r0, [r2]

    1:
        b 1b
        ",
        hf_lr = sym HF_LR,
        hf_msp = sym HF_MSP,
        hf_psp = sym HF_PSP,
        hf_frame = sym HF_FRAME,
    )
}
