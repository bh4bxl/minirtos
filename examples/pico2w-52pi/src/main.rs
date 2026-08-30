#![no_std]
#![no_main]

extern crate alloc;

mod drivers;
mod early_init;

use alloc::boxed::Box;
use cortex_m_rt::entry;
use defmt_rtt as _;
use minirtos_services::driver::{Uart, UartId, uart::UartConfig};
use panic_probe as _;

use minirtos_abi::{IpcMessageKind, MessageData, ServiceId};
use minirtos_kernel::{
    KernelConfig,
    sys::{self, Event, Mutex, Semaphore, Service, Write},
    task,
};

#[entry]
fn main() -> ! {
    defmt::info!(
        "miniRTOS {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    );

    minirtos_kernel::init_heap();

    let mut config = KernelConfig::new();

    match early_init::early_init(&mut config) {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Board {} early initialized.", env!("CARGO_PKG_NAME"),),
    }

    match minirtos_kernel::init(&config) {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Kernel start."),
    }

    drivers::init_driver_services();

    let sync = Box::leak(Box::new(SyncTest {
        sem: Semaphore::new(0).unwrap(),
        mutex: Mutex::new().unwrap(),
        event: Event::new(false).unwrap(),
    }));

    let _task = task::Task::new(default0)
        .arg(sync as *mut SyncTest as *mut ())
        .stack_size(1024)
        .priority(task::Priority(100))
        .spawn()
        .unwrap();
    let _task = task::Task::new(default1)
        .arg(sync as *mut SyncTest as *mut ())
        .stack_size(1024)
        .priority(task::Priority(100))
        .spawn()
        .unwrap();

    minirtos_kernel::start();
}

struct SyncTest {
    sem: Semaphore,
    mutex: Mutex,
    event: Event,
}

const TEST_SERVICE: ServiceId = ServiceId::from_raw(10);
const UART0: ServiceId = ServiceId::from_raw(20);

// Test Task
extern "C" fn default0(arg: *mut ()) {
    let sync = unsafe { &*(arg as *const SyncTest) };

    // test: sleep_ms, get_tick
    defmt::info!("-- task 0 normal test --");
    for i in 0..5 {
        defmt::info!("task 0: {} ({})", i, sys::get_tick());
        sys::sleep_ms(1000);
    }

    // Semaphore test
    defmt::info!("-- task 0 semaphore test --");
    defmt::info!("task 0 signal semaphore");
    sync.sem.release().unwrap();

    // Mutex test
    defmt::info!("-- task 0 mutex test --");
    {
        let _guard = sync.mutex.lock().unwrap();
        defmt::info!("task 0 acquired mutex");
        for i in 0..2 {
            defmt::info!("task 0 mutex: {}", i);
            sys::sleep_ms(500);
        }
    }

    // Event test
    defmt::info!("-- task 0 event test --");
    defmt::info!("task 0 enter event test");
    for i in 0..3 {
        defmt::info!("task 0 event: {}", i);
        sys::sleep_ms(1000);
    }

    // Service test
    defmt::info!("-- task 0 service test --");
    sys::sleep_ms(1000);
    //let endpoint = Endpoint::create().unwrap();
    let service = Service::new(TEST_SERVICE).unwrap();
    service.register().unwrap();
    defmt::info!("task 0 service registered");

    // Tell task1 that the service is ready.
    defmt::info!("task 0 signal event");
    sync.event.signal().unwrap();

    // Wait for message through the service endpoint.
    defmt::info!("task 0 waiting service message");
    let request = service.recv().unwrap();
    match request.kind {
        IpcMessageKind::Data => {
            defmt::info!(
                "task 0 received service message: sender={}, id={}, args=[{}, {}, {}, {}]",
                request.sender.raw(),
                request.op,
                request.args[0],
                request.args[1],
                request.args[2],
                request.args[3],
            );
        }

        IpcMessageKind::Write => {
            defmt::info!(
                "task 0 received write request: sender={}, op={}, ptr={:#x}, len={}",
                request.sender.raw(),
                request.op,
                request.ptr,
                request.len,
            );
        }

        IpcMessageKind::Read => {
            defmt::info!(
                "task 0 received read request: sender={}, op={}, ptr={:#x}, len={}",
                request.sender.raw(),
                request.op,
                request.ptr,
                request.len,
            );
        }
    }
    service.unregister().unwrap();
    defmt::info!("task 0 service unregistered");

    defmt::info!("task 0 exit");
}

extern "C" fn default1(arg: *mut ()) {
    let sync = unsafe { &*(arg as *const SyncTest) };

    defmt::info!("== task 1 normal test ==");
    for i in 0..5 {
        defmt::info!("task 1: {}", i);
        sys::sleep_ms(1500);
    }

    // Semaphore test
    defmt::info!("== task 1 semaphore test ==");
    defmt::info!("task 1 waiting semaphore");
    sync.sem.acquire().unwrap();

    // Mutex test
    defmt::info!("== task 1 enter mutex test ==");
    {
        let _guard = sync.mutex.lock().unwrap();
        defmt::info!("task 1 acquired mutex");
        for i in 0..2 {
            defmt::info!("task 1 mutex: {}", i);
            sys::sleep_ms(500);
        }
        defmt::info!("task 1 unlock mutex");
    }

    // Event test
    defmt::info!("== task 1 enter event test ==");
    defmt::info!("task 1 waiting event");
    sync.event.wait().unwrap();
    defmt::info!("task 1 event received");

    // Service test
    defmt::info!("== task 1 service test ==");
    let endpoint = Service::lookup(TEST_SERVICE).unwrap();
    defmt::info!("task 1 service found");
    let message = MessageData::new(1, [10, 20, 30, 40]);
    endpoint.send(&message).unwrap();
    defmt::info!("task 1 service message sent");

    // Uart test
    defmt::info!("== task 1 uart test ==");
    let mut uart = Uart::open(UartId::new(20)).unwrap();
    let config = UartConfig::default();
    uart.config(&config).unwrap();
    uart.write_all(b"hello").unwrap();

    defmt::info!("task 1 exit");
}
