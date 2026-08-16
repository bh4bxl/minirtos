use core::sync::atomic::{AtomicU32, Ordering};

use crate::{sched, synchronization::critical_section};

static TICK_HZ: AtomicU32 = AtomicU32::new(1000);

pub(crate) fn init(tick_hz: u32) {
    assert!(tick_hz > 0);

    TICK_HZ.store(tick_hz, Ordering::Relaxed);
}

#[inline]
pub fn tick() {
    critical_section(|cs| {
        sched::scheduler().update_tick(cs);
    });
}

#[inline]
pub fn get_sys_tick() -> u64 {
    critical_section(|cs| sched::scheduler().get_tick(cs))
}

#[inline]
pub fn tick_hz() -> u32 {
    TICK_HZ.load(Ordering::Relaxed)
}

#[inline]
pub(crate) fn ms_to_ticks(ms: u32) -> u64 {
    if ms == 0 {
        return 0;
    }

    let hz = tick_hz() as u64;

    // Round up so a non-zero sleep is never shorter than requested.
    ((ms as u64 * hz + 999) / 1000).max(1)
}
