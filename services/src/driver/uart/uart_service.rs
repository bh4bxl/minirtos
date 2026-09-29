use minirtos_abi::{ReceivedRequest, ServiceId, SharedBufferHandle, SysError};
use minirtos_kernel::interface::service::Service;
use minirtos_kernel::sys::{ServiceEndpoint, SharedBuffer};
use minirtos_kernel::{MemoryBlock, interface::driver::Driver};

use super::{super::DriverService, UartConfig, UartOp, interface::UartDriver};

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
    pub fn new(id: ServiceId, driver: DRV) -> Result<Self, SysError> {
        driver.init().map_err(|_| SysError::DeviceError)?;

        Ok(Self { id, driver })
    }

    pub fn run_loop(&mut self) -> ! {
        let service = ServiceEndpoint::new(self.id).unwrap();

        service.register().unwrap();

        loop {
            let request = service.recv().unwrap();

            self.handle_request(&service, request);
        }
    }

    fn handle_request(&mut self, service: &ServiceEndpoint, request: ReceivedRequest) {
        let Ok(op) = UartOp::try_from(request.op) else {
            return;
        };

        match op {
            UartOp::WriteByte => {
                let byte = request.args[0] as u8;

                let _ = self.driver.write_byte(byte);
            }

            UartOp::TryReadByte => {
                let result = match self.driver.try_read_byte() {
                    Ok(Some(byte)) => Ok(byte as u32),
                    Ok(None) => Err(SysError::WouldBlock),
                    Err(_) => Err(SysError::DeviceError),
                };

                let _ = service.complete(request.sender, result);
            }

            UartOp::ReadByte => {
                let result = match self.driver.read_byte() {
                    Ok(byte) => Ok(byte as u32),
                    Err(_) => Err(SysError::DeviceError),
                };

                let _ = service.complete(request.sender, result);
            }

            UartOp::Write => {
                let result = self.handle_write(&request);
                let _ = service.complete(request.sender, result);
            }

            UartOp::Read => {
                let result = self.handle_read(&request);
                let _ = service.complete(request.sender, result);
            }

            UartOp::Config => {
                let result = self.handle_config(&request);
                let _ = service.complete(request.sender, result);
            }

            UartOp::Flush => {
                let result = self
                    .driver
                    .flush()
                    .map(|_| 0)
                    .map_err(|_| SysError::DeviceError);

                let _ = service.complete(request.sender, result);
            }
        }
    }

    fn handle_write(&self, request: &ReceivedRequest) -> Result<u32, SysError> {
        let handle = SharedBufferHandle::from_raw(request.args[0]);
        let offset = request.args[1] as usize;
        let size = request.args[2] as usize;

        let shared = SharedBuffer::map(handle)?;

        let end = match offset.checked_add(size) {
            Some(end) if end <= shared.len() => end,
            _ => {
                let _ = shared.unmap();
                return Err(SysError::InvalidArgument);
            }
        };

        let data = &shared.as_slice()[offset..end];

        self.driver
            .write_buf(data)
            .map_err(|_| SysError::DeviceError)?;

        shared.unmap()?;

        Ok(0)
    }

    fn handle_read(&self, _request: &ReceivedRequest) -> Result<u32, SysError> {
        Ok(0)
    }

    fn handle_config(&self, request: &ReceivedRequest) -> Result<u32, SysError> {
        let handle = SharedBufferHandle::from_raw(request.args[0]);
        let offset = request.args[1] as usize;
        let size = request.args[2] as usize;

        if size != core::mem::size_of::<UartConfig>() {
            return Err(SysError::InvalidArgument);
        }

        let shared = SharedBuffer::map(handle)?;

        let end = offset.checked_add(size).ok_or(SysError::InvalidArgument)?;

        if end > shared.len() {
            let _ = shared.unmap();
            return Err(SysError::InvalidArgument);
        }

        let config = unsafe { (shared.as_ptr().add(offset) as *const UartConfig).read_unaligned() };

        self.driver
            .config(&config)
            .map(|_| 0)
            .map_err(|_| SysError::DeviceError)?;

        shared.unmap()?;

        Ok(0)
    }
}

impl<D> DriverService for UartService<D>
where
    D: UartDriver + Driver,
{
    fn device_memory_blocks(&self) -> &[MemoryBlock] {
        self.driver.device_memory_blocks()
    }
}

impl<D> Service for UartService<D>
where
    D: UartDriver + Driver,
{
    fn run(&mut self) -> ! {
        UartService::run_loop(self)
    }
}
