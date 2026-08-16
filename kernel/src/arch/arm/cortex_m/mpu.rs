#[derive(Debug, Clone, Copy, Default)]
pub struct ProtectionContext;

#[inline]
pub fn apply(_context: &ProtectionContext) {
    // TODO: Configure Cortex-M MPU regions.
}

#[inline]
pub fn clear() {
    // TODO: Disable/reset thread-specific MPU configuration.
}
