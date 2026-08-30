use minirtos_abi::{IpcMessageKind, ReceivedRequest, ServiceId, SysError};
use minirtos_kernel::sys::Service;

use super::{super::interface::Driver, UartConfig};

use super::{super::DriverService, UartOp, interface::UartDriver};

pub struct UartService<DRV>
where
    DRV: UartDriver,
{
    id: ServiceId,
    driver: DRV,
}

impl<DRV> UartService<DRV>
where
    DRV: UartDriver + Driver,
{
    pub fn new(id: ServiceId, mut driver: DRV) -> Result<Self, SysError> {
        driver.init().map_err(|_| SysError::DeviceError)?;

        Ok(Self { id, driver })
    }

    pub fn run_loop(&mut self) -> ! {
        let service = Service::new(self.id).unwrap();

        service.register().unwrap();

        loop {
            let request = service.recv().unwrap();

            self.handle_request(&service, request);
        }
    }

    fn handle_request(&mut self, service: &Service, request: ReceivedRequest) {
        let Ok(op) = UartOp::try_from(request.op) else {
            return;
        };

        match request.kind {
            IpcMessageKind::Data => match op {
                UartOp::WriteByte => {
                    let byte = request.args[0] as u8;
                    let _ = self.driver.write_byte(byte);
                }

                UartOp::TryReadByte => {
                    let _ = self.driver.try_read_byte();
                }

                UartOp::Write | UartOp::Read | UartOp::Config => return,
            },

            IpcMessageKind::Write => {
                let result = match op {
                    UartOp::Write => {
                        let buf = unsafe {
                            core::slice::from_raw_parts(request.ptr as *const u8, request.len)
                        };

                        self.driver
                            .write_buf(buf)
                            .map_err(|_| SysError::DeviceError)
                    }

                    UartOp::Config => {
                        if request.len != core::mem::size_of::<UartConfig>() {
                            Err(SysError::InvalidArgument)
                        } else {
                            let config = unsafe { (request.ptr as *const UartConfig).read() };

                            self.driver
                                .config(&config)
                                .map(|_| 0)
                                .map_err(|_| SysError::DeviceError)
                        }
                    }

                    _ => Err(SysError::InvalidArgument),
                };

                service.complete(request.sender, result).unwrap();
            }

            IpcMessageKind::Read => {
                if !matches!(op, UartOp::Read) {
                    return;
                }
            }
        }
    }
}

impl<D> DriverService for UartService<D>
where
    D: UartDriver + Driver,
{
    fn run(&mut self) -> ! {
        UartService::run_loop(self)
    }
}
