use defmt::info;

use crate::sys::{
    interrupt::irq_manager,
    synchronization::{IrqSafeNullLock, interface::Mutex},
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeviceType {
    Uart,
    Gpio,
    Other,
}

/// Driver interface
pub mod interface {

    /// Device driver
    pub trait DeviceDriver {
        type IrqNumberType: core::fmt::Debug;

        /// Return a compatibility string
        fn compatible(&self) -> &'static str;

        /// Bring up the device
        fn init(&self) -> Result<(), &'static str> {
            Ok(())
        }

        /// Register IRQ handler
        fn register_irq_handler(
            &'static self,
            irq_number: Self::IrqNumberType,
        ) -> Result<(), &'static str> {
            panic!(
                "Attempt to enable IRQ {:?} for device {}, but driver does not support this",
                irq_number,
                self.compatible()
            );
        }
    }
}

/// Callback after a driver's init()
pub type DeviceDriverPostInitCallback = fn() -> Result<(), &'static str>;

/// A descriptor for device drivers.
#[derive(Copy, Clone)]
pub struct DeviceDriverDescriptor<T>
where
    T: 'static,
{
    device_driver: &'static (dyn interface::DeviceDriver<IrqNumberType = T> + Sync),
    post_init_callback: Option<DeviceDriverPostInitCallback>,
    irq_number: Option<T>,
}

impl<T> DeviceDriverDescriptor<T> {
    /// Create an instance.
    pub fn new(
        device_driver: &'static (dyn interface::DeviceDriver<IrqNumberType = T> + Sync),
        post_init_callback: Option<DeviceDriverPostInitCallback>,
        irq_number: Option<T>,
    ) -> Self {
        Self {
            device_driver,
            post_init_callback,
            irq_number,
        }
    }
}

const NUM_DRIVERS: usize = 5;

struct DriverManagerInner<T>
where
    T: 'static,
{
    next_index: usize,
    descriptior: [Option<DeviceDriverDescriptor<T>>; NUM_DRIVERS],
}

impl<T> DriverManagerInner<T>
where
    T: 'static + Copy,
{
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            next_index: 0,
            descriptior: [None; NUM_DRIVERS],
        }
    }
}

/// Provides device driver management functions.
pub struct DriverManager<T>
where
    T: 'static,
{
    inner: IrqSafeNullLock<DriverManagerInner<T>>,
}

impl<T> DriverManager<T>
where
    T: core::fmt::Debug + Copy,
{
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            inner: IrqSafeNullLock::new(DriverManagerInner::new()),
        }
    }

    /// Register a device driver.
    pub fn register(&self, descripter: DeviceDriverDescriptor<T>) {
        self.inner.lock(|inner| {
            inner.descriptior[inner.next_index] = Some(descripter);
            inner.next_index += 1;
        })
    }

    /// Iterate over registered drivers.
    fn for_each_descriptor<'a>(&'a self, f: impl FnMut(&'a DeviceDriverDescriptor<T>)) {
        self.inner.lock(|inner| {
            inner
                .descriptior
                .iter()
                .filter_map(|x| x.as_ref())
                .for_each(f)
        })
    }

    /// Fully initialize all drivers.
    pub unsafe fn init_drivers(&self) {
        self.for_each_descriptor(|descriptor| {
            // Initialize driver.
            if let Err(x) = descriptor.device_driver.init() {
                panic!(
                    "Error initializing driver: {}: {}",
                    descriptor.device_driver.compatible(),
                    x
                )
            }

            // Call corresponding post init callback.
            if let Some(callback) = descriptor.post_init_callback {
                if let Err(x) = callback() {
                    panic!(
                        "Error during driver post-init callback: {}: {}",
                        descriptor.device_driver.compatible(),
                        x
                    )
                }
            }
        });

        // registered IRQs
        irq_manager().enable(false);
        self.for_each_descriptor(|descriptor| {
            if let Some(irq_number) = descriptor.irq_number {
                if let Err(x) = descriptor.device_driver.register_irq_handler(irq_number) {
                    panic!(
                        "Error registering IRQ handler: {}: {}",
                        descriptor.device_driver.compatible(),
                        x
                    );
                }
            }
        });
        irq_manager().enable(true);
    }

    /// Enumerate all registered device drivers.
    pub fn enumerate(&self) {
        let mut i = 1usize;
        self.for_each_descriptor(|descriptor| {
            info!("      {}. {}", i, descriptor.device_driver.compatible());
            i += 1;
        });
    }
}

static DRIVER_MANAGER: DriverManager<super::interrupt::IrqNumber> = DriverManager::new();

/// A reference to the global DriverManager.
pub fn driver_manager() -> &'static DriverManager<super::interrupt::IrqNumber> {
    &DRIVER_MANAGER
}
