use cortex_m::peripheral::MPU;

use super::{Access, ProtectionContext, Region};

const CTRL_ENABLE: u32 = 1 << 0;
const CTRL_PRIVDEFENA: u32 = 1 << 2;

const RBAR_XN: u32 = 1 << 0;
const RBAR_AP_RW: u32 = 0b01 << 1;
const RBAR_AP_RO: u32 = 0b11 << 1;

const RLAR_ENABLE: u32 = 1;
const ATTR_NORMAL: u32 = 0;
const ATTR_DEVICE: u32 = 1;

pub(super) fn init() {
    let mpu = unsafe { &*MPU::PTR };

    unsafe {
        disable(mpu);
        clear_regions(mpu);

        // Attr0: Normal non-cacheable
        // Attr1: Device nGnRnE
        mpu.mair[0].write(0x44 | (0x00 << 8));
    }

    barrier();
}

pub(super) fn apply(context: &ProtectionContext) {
    let mpu = unsafe { &*MPU::PTR };

    disable(mpu);
    clear_regions(mpu);

    for (index, region) in context.regions.iter().enumerate() {
        if let Some(region) = region {
            configure_region(mpu, index, *region);
        }
    }

    enable(mpu);

    barrier();
}

pub(super) fn clear() {
    let mpu = unsafe { &*MPU::PTR };

    disable(mpu);
    clear_regions(mpu);
    enable(mpu);

    barrier();
}

fn configure_region(mpu: &cortex_m::peripheral::mpu::RegisterBlock, index: usize, region: Region) {
    let (ap, xn, attr) = match region.access {
        Access::ReadExecute => (RBAR_AP_RO, 0, ATTR_NORMAL),
        Access::ReadOnly => (RBAR_AP_RO, RBAR_XN, ATTR_NORMAL),
        Access::ReadWrite => (RBAR_AP_RW, RBAR_XN, ATTR_NORMAL),
        Access::DeviceReadWrite => (RBAR_AP_RW, RBAR_XN, ATTR_DEVICE),
    };

    let limit = region.base + region.size - 1;

    let rbar = (region.base as u32 & !0x1f) | ap | xn;

    let rlar = (limit as u32 & !0x1f) | (attr << 1) | RLAR_ENABLE;

    unsafe {
        mpu.rnr.write(index as u32);
        mpu.rbar.write(rbar);
        mpu.rlar.write(rlar);
    }
}

fn clear_regions(mpu: &cortex_m::peripheral::mpu::RegisterBlock) {
    let count = region_count(mpu);

    for index in 0..count {
        unsafe {
            mpu.rnr.write(index as u32);
            mpu.rlar.write(0);
        }
    }
}

fn region_count(mpu: &cortex_m::peripheral::mpu::RegisterBlock) -> usize {
    ((mpu._type.read() >> 8) & 0xff) as usize
}

fn disable(mpu: &cortex_m::peripheral::mpu::RegisterBlock) {
    unsafe {
        mpu.ctrl.write(0);
    }

    barrier();
}

fn enable(mpu: &cortex_m::peripheral::mpu::RegisterBlock) {
    unsafe {
        mpu.ctrl.write(CTRL_ENABLE | CTRL_PRIVDEFENA);
    }
}

#[inline(always)]
fn barrier() {
    unsafe {
        core::arch::asm!("dsb");
        core::arch::asm!("isb");
    }
}
