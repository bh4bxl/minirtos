/// Interrupt

pub mod interface {
    pub trait IrqHandler {
        fn handler(&self) -> Result<(), &'static str>;
    }
}

/// Interrupt descriptor.
#[derive(Copy, Clone)]
pub struct IrqHandlerDescriptor<T>
where
    T: Copy,
{
    /// The IRQ number.
    pub irq_number: T,

    /// Descriptive name.
    name: &'static str,

    /// Reference to handler trait object.
    handler: &'static (dyn interface::IrqHandler + Sync),
}

impl<T> IrqHandlerDescriptor<T>
where
    T: Copy,
{
    /// Create an instance.
    pub const fn new(
        number: T,
        name: &'static str,
        handler: &'static (dyn interface::IrqHandler + Sync),
    ) -> Self {
        Self {
            irq_number: number,
            name,
            handler,
        }
    }

    /// Return the number.
    pub const fn number(&self) -> T {
        self.irq_number
    }

    /// Return the name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Return the handler.
    pub const fn handler(&self) -> &'static (dyn interface::IrqHandler + Sync) {
        self.handler
    }
}
