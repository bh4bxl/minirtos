use crate::sys::synchronization::{InitStateLock, interface::ReadWriteEx};

pub type IrqNumber = rp235x_pac::Interrupt;

/// Interrupt interface.
pub mod interface {
    /// Implemented by types that handle IRQs.
    pub trait IrqHandler {
        /// Called when the corresponding interrupt is asserted.
        fn handler(&self) -> Result<(), &'static str>;
    }

    /// IRQ management functions.
    pub trait IrqManager {
        /// The IRQ number type depends on the implementation.
        type IrqNumberType: Copy;

        /// Register a handler.
        fn register_irq_handler(
            &self,
            irq_handler_decsriptor: super::IrqHandlerDescriptor<Self::IrqNumberType>,
        ) -> Result<(), &'static str>;

        /// Enable the controller.
        fn enable(&self, enable: bool);

        /// Dispatch an IRQ.
        fn dispatch(&self, irq_number: Self::IrqNumberType) -> Result<(), &'static str>;

        /// Enumerate all registered IRQs.
        fn enumerate(&self);
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

/// A placeholder.
struct NullIrqManager;

impl interface::IrqManager for NullIrqManager {
    type IrqNumberType = IrqNumber;

    fn register_irq_handler(
        &self,
        _irq_handler_decsriptor: self::IrqHandlerDescriptor<Self::IrqNumberType>,
    ) -> Result<(), &'static str> {
        panic!("No IRQ Manager registered yet");
    }

    fn enable(&self, _enable: bool) {
        panic!("No IRQ Manager registered yet");
    }

    fn dispatch(&self, _irq_number: Self::IrqNumberType) -> Result<(), &'static str> {
        panic!("No IRQ Manager registered yet");
    }

    fn enumerate(&self) {}
}

static NULL_IRQ_MANAGER: NullIrqManager = NullIrqManager {};

static CURR_IRQ_MANAGER: InitStateLock<
    &'static (dyn interface::IrqManager<IrqNumberType = IrqNumber> + Sync),
> = InitStateLock::new(&NULL_IRQ_MANAGER);

/// Register a new IRQ manager.
pub fn register_irq_manager(
    new_manager: &'static (dyn interface::IrqManager<IrqNumberType = IrqNumber> + Sync),
) {
    CURR_IRQ_MANAGER.write(|manager| *manager = new_manager);
}

pub fn irq_manager() -> &'static dyn interface::IrqManager<IrqNumberType = IrqNumber> {
    CURR_IRQ_MANAGER.read(|manager| *manager)
}
