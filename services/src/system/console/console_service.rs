use minirtos_abi::{ReceivedRequest, ServiceId, SharedBufferHandle, SysError};
use minirtos_kernel::{
    interface::{IoAll, service::Service},
    sys::{ServiceEndpoint, SharedBuffer},
};

use super::ConsoleOp;

pub struct ConsoleService<'a> {
    id: ServiceId,

    backend: Option<&'a (dyn IoAll + Sync)>,

    tx: Option<SharedBuffer>,
    rx: Option<SharedBuffer>,
}

impl<'a> ConsoleService<'a> {
    pub fn new(id: ServiceId) -> Self {
        Self {
            id,
            backend: None,
            tx: None,
            rx: None,
        }
    }

    pub fn set_backend(&mut self, backend: &'a (dyn IoAll + Sync)) {
        self.backend = Some(backend);
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
        let Ok(op) = ConsoleOp::try_from(request.op) else {
            return;
        };

        match op {
            ConsoleOp::Register => {
                let result = self.handle_register(&request);
                let _ = service.complete(request.sender, result);
            }

            ConsoleOp::WritePending => {
                let result = self.handle_write(&request);
                let _ = service.complete(request.sender, result);
            }

            ConsoleOp::WaitRead => {}
            ConsoleOp::ClearInput => {}

            ConsoleOp::Flush => {
                if let Some(backend) = self.backend {
                    backend.flush();
                    let _ = service.complete(request.sender, Ok(0));
                } else {
                    let _ = service.complete(request.sender, Err(SysError::InvalidState));
                }
            }
        }
    }

    fn handle_register(&mut self, request: &ReceivedRequest) -> Result<u32, SysError> {
        if self.tx.is_some() || self.rx.is_some() {
            return Err(SysError::AlreadyExists);
        }

        let tx_handle = SharedBufferHandle::from_raw(request.args[0]);
        let rx_handle = SharedBufferHandle::from_raw(request.args[1]);

        let tx = SharedBuffer::map(tx_handle)?;

        let rx = match SharedBuffer::map(rx_handle) {
            Ok(rx) => rx,
            Err(err) => {
                let _ = tx.unmap();
                return Err(err);
            }
        };

        self.tx = Some(tx);
        self.rx = Some(rx);

        Ok(0)
    }

    fn handle_write(&self, request: &ReceivedRequest) -> Result<u32, SysError> {
        let backend = self.backend.ok_or(SysError::InvalidState)?;

        let tx = self.tx.as_ref().ok_or(SysError::InvalidState)?;

        let size = request.args[0] as usize;

        if size > tx.len() {
            return Err(SysError::InvalidArgument);
        }

        let data = unsafe { core::slice::from_raw_parts(tx.as_ptr(), size) };

        let s = core::str::from_utf8(data).map_err(|_| SysError::InvalidArgument)?;

        backend
            .write_fmt(format_args!("{}", s))
            .map_err(|_| SysError::Io)?;

        Ok(size as u32)
    }
}

impl<'a> Service for ConsoleService<'a> {
    fn run(&mut self) -> ! {
        self.run_loop()
    }
}
