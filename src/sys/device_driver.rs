use defmt::info;

use crate::sys::{
    interrupt::irq_manager,
    synchronization::{IrqSafeNullLock, interface::Mutex},
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DeviceType {
    Uart = 0,
    Gpio,
    Count,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum DevError {
    Busy,
    NoDevice,
    Unsupported,
    WouldBlock,
    Timeout,
    InvalidArg,
    Io,
}

/// Driver interface
pub mod interface {

    /// Device driver
    pub trait Driver {
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

    /// Device interface
    pub trait Device {
        /// Read data from the device
        fn read(&self, _data: &mut [u8]) -> Result<usize, super::DevError>;

        /// Write data to the device
        fn write(&self, _data: &[u8]) -> Result<usize, super::DevError>;
    }

    pub trait DeviceDriver: Driver + Device {
        fn as_device(&self) -> &dyn Device;
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
    device_type: DeviceType,
}

impl<T> DeviceDriverDescriptor<T> {
    /// Create an instance.
    pub fn new(
        device_driver: &'static (dyn interface::DeviceDriver<IrqNumberType = T> + Sync),
        post_init_callback: Option<DeviceDriverPostInitCallback>,
        irq_number: Option<T>,
        device_type: DeviceType,
    ) -> Self {
        Self {
            device_driver,
            post_init_callback,
            irq_number,
            device_type,
        }
    }
}

const NUM_DRIVERS: usize = 5;

struct DriverManagerInner<T>
where
    T: 'static,
{
    next_index: usize,
    descriptor: [Option<DeviceDriverDescriptor<T>>; NUM_DRIVERS],
    busy_devices: [bool; NUM_DRIVERS],
}

impl<T> DriverManagerInner<T>
where
    T: 'static + Copy,
{
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            next_index: 0,
            descriptor: [None; NUM_DRIVERS],
            busy_devices: [false; NUM_DRIVERS],
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

pub struct DeviceHandler<'a, T>
where
    T: 'static + Copy,
{
    index: usize,
    manager: &'a DriverManager<T>,
}

impl<'a, T> core::ops::Deref for DeviceHandler<'a, T>
where
    T: 'static + core::fmt::Debug + Copy,
{
    type Target = dyn interface::Device;

    fn deref(&self) -> &Self::Target {
        self.manager.get_device(self.index).unwrap()
    }
}

impl<'a, T> Drop for DeviceHandler<'a, T>
where
    T: 'static + Copy,
{
    fn drop(&mut self) {
        self.manager
            .inner
            .lock(|inner| inner.busy_devices[self.index] = false);
    }
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
            inner.descriptor[inner.next_index] = Some(descripter);
            inner.next_index += 1;
        })
    }

    /// Iterate over registered drivers.
    fn for_each_descriptor<'a>(&'a self, f: impl FnMut(&'a DeviceDriverDescriptor<T>)) {
        self.inner.lock(|inner| {
            inner
                .descriptor
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

    /// Open a device.
    pub fn open_device(
        &self,
        device_type: DeviceType,
        index: usize,
    ) -> Option<DeviceHandler<'_, T>> {
        let mut cnt = 0;
        let mut found_device = false;
        for i in 0..NUM_DRIVERS {
            if let Some(descriptor) = self.inner.lock(|inner| inner.descriptor[i]) {
                if descriptor.device_type == device_type {
                    if cnt == index {
                        if self.inner.lock(|inner| inner.busy_devices[cnt]) {
                            return None;
                        }
                        self.inner.lock(|inner| inner.busy_devices[cnt] = true);
                        found_device = true;
                        break;
                    }
                    cnt += 1;
                }
            }
        }
        if found_device {
            return Some(DeviceHandler {
                index: cnt,
                manager: self,
            });
        }
        None
    }

    pub fn get_device(&self, index: usize) -> Option<&'static dyn interface::Device> {
        self.inner.lock(|inner| {
            if let Some(descriptor) = inner.descriptor[index] {
                return Some(descriptor.device_driver.as_device());
            }
            None
        })
    }
}

static DRIVER_MANAGER: DriverManager<super::interrupt::IrqNumber> = DriverManager::new();

/// A reference to the global DriverManager.
pub fn driver_manager() -> &'static DriverManager<super::interrupt::IrqNumber> {
    &DRIVER_MANAGER
}
