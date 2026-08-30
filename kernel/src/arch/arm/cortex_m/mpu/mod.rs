mod context;
mod registers;

pub use context::{Access, ProtectionContext, ProtectionError, Region};

pub(crate) const MAX_REGIONS: usize = 8;

#[inline]
pub fn init() {
    registers::init();
}

#[inline]
pub fn apply(context: &ProtectionContext) {
    registers::apply(context);
}

#[inline]
pub fn clear() {
    registers::clear();
}
