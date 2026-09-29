#![no_std]
#![no_main]

extern crate alloc;

mod drivers;
mod early_init;
mod services;

use alloc::boxed::Box;
use cortex_m_rt::entry;
use defmt_rtt as _;
use minirtos_services::driver::{Uart, UartId, uart::UartConfig};
use panic_probe as _;

use minirtos_abi::{MessageData, Priority, ServiceId, SharedBufferHandle, SysError};
use minirtos_kernel::{
    KernelConfig,
    interface::IoRead,
    print, println,
    sys::{self, Event, Mutex, Semaphore, ServiceEndpoint, SharedBuffer},
    task,
};

#[unsafe(no_mangle)]
unsafe fn drivers_init() -> Result<(), SysError> {
    drivers::init_driver_services();

    Ok(())
}

#[unsafe(no_mangle)]
unsafe fn services_init() -> Result<(), SysError> {
    services::init_system_service()?;

    Ok(())
}

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

    println!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    match minirtos_kernel::init(&config) {
        Err(e) => {
            defmt::error!("Error: {:?}", e as u16);
            panic!("early init failed");
        }
        Ok(()) => defmt::info!("Kernel start."),
    }

    minirtos_kernel::start();
}

#[unsafe(no_mangle)]
unsafe fn user_apps_init() -> Result<(), SysError> {
    let sync = Box::leak(Box::new(SyncTest {
        sem: Semaphore::new(0).unwrap(),
        mutex: Mutex::new().unwrap(),
        event: Event::new(false).unwrap(),
    }));

    let _task = task::Task::new(default0)
        .arg(sync as *mut SyncTest as *mut ())
        .stack_size(1024)
        .priority(Priority(100))
        .spawn()?;
    let _task = task::Task::new(default1)
        .arg(sync as *mut SyncTest as *mut ())
        .stack_size(1024)
        .priority(Priority(100))
        .spawn()?;
    Ok(())
}

struct SyncTest {
    sem: Semaphore,
    mutex: Mutex,
    event: Event,
}

const TEST_SERVICE: ServiceId = ServiceId::from_raw(10);

// Test Task
extern "C" fn default0(arg: *mut ()) {
    let sync = unsafe { &*(arg as *const SyncTest) };

    // test: sleep_ms, get_tick
    println!("-- task 0 normal test --");
    for i in 0..5 {
        println!("task 0: {} ({})", i, sys::get_tick());
        sys::sleep_ms(1000);
    }

    // Semaphore test
    println!("-- task 0 semaphore test --");
    println!("task 0 signal semaphore");
    sync.sem.release().unwrap();

    // Mutex test
    println!("-- task 0 mutex test --");
    {
        let _guard = sync.mutex.lock().unwrap();
        println!("task 0 acquired mutex");
        for i in 0..2 {
            println!("task 0 mutex: {}", i);
            sys::sleep_ms(500);
        }
    }

    // Event test
    println!("-- task 0 event test --");
    println!("task 0 enter event test");
    for i in 0..3 {
        println!("task 0 event: {}", i);
        sys::sleep_ms(1000);
    }

    // Service test
    println!("-- task 0 service test --");
    sys::sleep_ms(1000);
    //let endpoint = Endpoint::create().unwrap();
    let service = ServiceEndpoint::new(TEST_SERVICE).unwrap();
    service.register().unwrap();
    println!("task 0 service registered");

    // Tell task1 that the service is ready.
    println!("task 0 signal event");
    sync.event.signal().unwrap();

    // Wait for message through the service endpoint.
    println!("task 0 waiting service message");
    let request = service.recv().unwrap();

    println!(
        "task 0 received service message: sender={}, id={}, args=[{}, {}, {}, {}]",
        request.sender.raw(),
        request.op,
        request.args[0],
        request.args[1],
        request.args[2],
        request.args[3],
    );

    service.unregister().unwrap();
    println!("task 0 service unregistered");

    // Shared Memory test
    println!("-- task 0 shared buffer test --");
    let handle = SharedBufferHandle::from_raw(request.args[0]);
    let mut buffer = SharedBuffer::map(handle).unwrap();
    for (i, byte) in buffer.as_slice().iter().enumerate() {
        assert_eq!(*byte, i as u8);
    }
    println!("shared buffer[127] = 0x{:X}", buffer.as_slice()[127]);
    buffer.as_mut_slice()[0] = 0x55;
    buffer.unmap().unwrap();

    println!("<task 0 exit>");
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

    // Create hared memory
    let mut buffer = SharedBuffer::alloc(128).unwrap();
    defmt::info!(
        "shared buffer: ptr=0x{:X}, size={}",
        buffer.as_ptr() as u32,
        buffer.len()
    );
    let data = buffer.as_mut_slice();
    for (i, byte) in data.iter_mut().enumerate() {
        *byte = i as u8;
    }
    let handle = buffer.handle();

    // Service test
    defmt::info!("== task 1 service test ==");
    let endpoint = ServiceEndpoint::lookup(TEST_SERVICE).unwrap();
    defmt::info!("task 1 service found");
    let message = MessageData::new(1, [handle.raw(), 20, 30, 40]);
    endpoint.send(&message).unwrap();
    defmt::info!("task 1 service message sent");

    defmt::info!("== task 1 shared buffer test ==");
    sys::sleep_ms(1000);
    defmt::info!("shared buffer[0] = 0x{:X}", buffer.as_slice()[0]);
    assert_eq!(buffer.as_slice()[0], 0x55);

    // Uart test
    defmt::info!("== task 1 uart test ==");
    let uart = Uart::open(UartId::new(services::DRV_UART0.raw())).unwrap();
    let config = UartConfig::default();
    uart.config(&config).unwrap();
    println!("hello, miniRTOS");
    println!("Input:");

    loop {
        let c = uart.read_char();
        print!("{}", c);
        if c == 'q' {
            break;
        }
        sys::sleep_ms(100);
    }

    defmt::info!("<task 1 exit>");
}
