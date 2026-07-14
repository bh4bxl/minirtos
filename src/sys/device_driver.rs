use alloc::vec::Vec;

use crate::{
    m_info, println,
    sys::{
        interrupt::irq_manager,
        synchronization::{NullLock, interface::Mutex},
    },
};

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DeviceType {
    Uart = 0,
    Gpio,
    Spi,
    I2c,
    Lcd,
    Input,
    Wlan,
    Bluetooth,
    Count,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DevError {
    Busy,
    NoSuchDevice,
    Unsupported,
    WouldBlock,
    Timeout,
    InvalidArg,
    Io,
    DevAlreadyInit,
    NoFreeDriverSlot,
    NoMem,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceIrqEvent {
    Gpio,
    RxReady,
    TxReady,
    Error,
}

#[derive(Clone, Copy)]
pub struct DeviceIrq {
    pub event: DeviceIrqEvent,
    pub data: usize,
}

pub type DeviceIrqCallback = fn(DeviceIrq);

/// Driver interface
pub mod interface {
    use crate::sys::device_driver::DevError;

    /// Device driver
    pub trait Driver {
        type IrqNumberType: core::fmt::Debug;

        /// Return a compatibility string
        fn compatible(&self) -> &'static str;

        /// Bring up the device
        fn init(&self) -> Result<(), DevError> {
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

    #[allow(dead_code)]
    /// Device interface
    pub trait Device {
        /// Read data from the device
        fn read(&self, _data: &mut [u8]) -> Result<usize, super::DevError>;

        /// Write data to the device
        fn write(&self, _data: &[u8]) -> Result<usize, super::DevError>;

        fn set_irq_callback(
            &self,
            _callback: Option<super::DeviceIrqCallback>,
        ) -> Result<(), DevError> {
            Err(super::DevError::Unsupported)
        }
    }

    pub trait DeviceDriver: Driver + Device {
        fn as_device(&self) -> &dyn Device;
    }
}

/// Callback after a driver's init()
pub type DeviceDriverPostInitCallback = fn() -> Result<(), DevError>;

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

struct DriverEntry<T>
where
    T: 'static,
{
    descriptor: DeviceDriverDescriptor<T>,
    busy: bool,
}

struct DriverManagerInner<T>
where
    T: 'static,
{
    drivers: Vec<DriverEntry<T>>,
}

impl<T> DriverManagerInner<T>
where
    T: 'static + Copy,
{
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }
}

/// Provides device driver management functions.
pub struct DriverManager<T>
where
    T: 'static,
{
    inner: NullLock<DriverManagerInner<T>>,
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
        self.manager.inner.lock(|inner| {
            if let Some(entry) = inner.drivers.get_mut(self.index) {
                entry.busy = false;
            }
        });
    }
}

impl<T> DriverManager<T>
where
    T: core::fmt::Debug + Copy,
{
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            inner: NullLock::new(DriverManagerInner::new()),
        }
    }

    /// Register a device driver.
    pub fn register(&self, descriptor: DeviceDriverDescriptor<T>) -> Result<(), DevError> {
        self.inner.lock(|inner| {
            inner.drivers.push(DriverEntry {
                descriptor,
                busy: false,
            });
            Ok(())
        })
    }

    /// Iterate over registered drivers.
    fn for_each_descriptor<'a>(&'a self, f: impl FnMut(&'a DeviceDriverDescriptor<T>)) {
        self.inner
            .lock(|inner| inner.drivers.iter().map(|x| &x.descriptor).for_each(f))
    }

    /// Fully initialize all drivers.
    pub unsafe fn init_drivers(&self) {
        self.for_each_descriptor(|descriptor| {
            // Initialize driver.
            if let Err(x) = descriptor.device_driver.init() {
                panic!(
                    "Error initializing driver: {}: {:?}",
                    descriptor.device_driver.compatible(),
                    x
                )
            }

            // Call corresponding post init callback.
            if let Some(callback) = descriptor.post_init_callback {
                if let Err(x) = callback() {
                    panic!(
                        "Error during driver post-init callback: {}: {:?}",
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
            m_info!("      {}. {}", i, descriptor.device_driver.compatible());
            i += 1;
        });
    }

    /// Dump devices
    pub fn dump_device(&self) {
        let mut i = 1usize;
        self.for_each_descriptor(|descriptor| {
            println!("      {}. {}", i, descriptor.device_driver.compatible());
            i += 1;
        });
    }

    /// Open a device.
    pub fn open_device(
        &self,
        device_type: DeviceType,
        index: usize,
    ) -> Option<DeviceHandler<'_, T>> {
        self.inner.lock(|inner| {
            let mut count = 0;

            for (driver_index, entry) in inner.drivers.iter_mut().enumerate() {
                if entry.descriptor.device_type != device_type {
                    continue;
                }

                if count == index {
                    if entry.busy {
                        return None;
                    }

                    entry.busy = true;

                    return Some(DeviceHandler {
                        index: driver_index,
                        manager: self,
                    });
                }

                count += 1;
            }

            None
        })
    }

    pub fn get_device(&self, index: usize) -> Option<&'static dyn interface::Device> {
        self.inner.lock(|inner| {
            inner
                .drivers
                .get(index)
                .map(|entry| entry.descriptor.device_driver.as_device())
        })
    }
}

static DRIVER_MANAGER: DriverManager<super::interrupt::IrqNumber> = DriverManager::new();

/// A reference to the global DriverManager.
pub fn driver_manager() -> &'static DriverManager<super::interrupt::IrqNumber> {
    &DRIVER_MANAGER
}
