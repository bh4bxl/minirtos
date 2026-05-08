# miniRTOS (RP2350, Rust)

A lightweight embedded RTOS and driver platform written in Rust for ARM Cortex-M33 systems based on the RP2350 (Raspberry Pi Pico 2 / Pico 2W).

This project explores low-level embedded systems development including task scheduling, interrupt handling, device drivers, embedded graphics, Wi-Fi integration, and networking infrastructure.

The project is intentionally built step-by-step from low-level hardware bring-up toward a more complete embedded runtime environment, with focus on system transparency, architecture clarity, and hands-on understanding of embedded internals.

---

## 🚧 Project Status

Work in progress.

---

## 🏗 Architecture Overview

```text
Applications / Tasks
          ↓
 Scheduler / IPC / Syscalls
          ↓
 Driver Framework
          ↓
 BSP / MCU Layer
          ↓
 Hardware
```

The project is structured to maintain separation between:

* platform-independent system logic
* reusable drivers
* MCU / BSP-specific code
* hardware integration

---

## ✅ Implemented Features

### Core System

* Basic RP2350 bring-up on Raspberry Pi Pico 2 / Pico 2W
* PendSV/SVC-based context switching
* Static task creation and basic scheduler
* SysTick-based system tick
* Interrupt registration and centralized IRQ dispatch
* ARM Cortex-M exception handling infrastructure

### Synchronization & IPC

* IRQ-safe critical section primitives
* Mutexes
* Semaphores
* Events
* Wait queues
* Message queues

### Driver Framework

* Modular driver framework (`driver_manager`)
* UART driver (PL011-based)
* GPIO driver
* SPI driver
* LCD driver framework

### Display & Graphics

* ST7789VW LCD driver support
* Framebuffer-based rendering path
* RGB565 rendering pipeline
* `embedded-graphics` DrawTarget integration
* SPI DMA acceleration for framebuffer flush
* Optimized 16-bit SPI DMA pixel transfer

### Networking

* smoltcp integration experiments
* Packet handling pipeline
* Network device abstraction
* ARP / ICMP packet experiments
* Embedded networking infrastructure for future TCP/IP support

### Wi-Fi Integration

* Experimental CYW43 Wi-Fi integration on Pico 2W
* Low-level SPI transport handling
* Firmware/NVRAM loading experiments
* Packet communication infrastructure
* PIO/SPI timing validation using logic analyzer

### Debugging & Instrumentation

* UART console abstraction
* Low-level debugging support
* Logic analyzer validation workflows
* UART tracing
* Embedded instrumentation using defmt

---

## 🚧 In Progress

* Scheduler refinement
* Preemptive scheduling improvements
* UART RX/TX ring buffer
* Software timers
* Driver abstraction cleanup
* Better BSP separation
* DMA abstraction improvements
* Optional LVGL integration

---

## 📌 Planned Features

* Dynamic task creation
* Memory allocator / heap support
* Priority-based scheduling
* Portable BSP abstraction
* Filesystem / storage support
* Expanded networking functionality
* Multi-platform support

---

## 🧠 Design Goals

* Keep the system architecture simple and transparent
* Build embedded subsystems incrementally from low-level primitives
* Avoid unnecessary abstractions during early development
* Improve understanding of RTOS internals and Cortex-M architecture
* Maintain clear separation between:

  * scheduler/runtime logic
  * drivers
  * BSP implementation
  * hardware-specific code

---

## 📁 Project Structure

```text
src/
├── apps/                 # Example applications
│   └── hmi/              # UI / display-related application layer
│
├── bsp/                  # Board support package
│   ├── boards/           # Board-level configuration
│   └── mcu/
│       └── rp235x/       # RP2350-specific MCU implementation
│
├── drivers/              # Device drivers
│   ├── gpio/
│   ├── input/
│   ├── lcd/
│   ├── spi/
│   ├── uart/
│   └── wlan/
│       └── cyw43/        # CYW43 Wi-Fi integration
│
├── gui/                  # Graphics and UI subsystem
│
├── net/                  # Networking infrastructure and smoltcp integration
│
└── sys/                  # Core runtime and RTOS infrastructure
    ├── arch/             # Architecture-specific runtime code
    ├── console/          # Console and logging infrastructure
    └── sync/             # Synchronization and IPC primitives
```

### Layering Philosophy

The project follows a layered architecture intended to separate:

* application logic
* RTOS/runtime infrastructure
* reusable drivers
* board support code
* hardware-specific implementation

This structure is designed to keep low-level platform code isolated while allowing higher-level components to remain portable and reusable.

---

## ⚙️ Interrupt Handling

* Centralized interrupt registration through `sys::interrupt`
* Descriptor-based IRQ registration
* NVIC integration for RP2350
* Cortex-M exception handling for:

  * SysTick
  * PendSV
  * SVC
  * HardFault

---

## 🔒 Synchronization & IPC

Current synchronization primitives include:

* IRQ-safe critical sections
* Mutex
* Semaphore
* Event
* WaitQueue
* MessageQueue

These are implemented with focus on deterministic embedded behavior and scheduler integration.

---

## 📡 Networking

Networking experiments currently use `smoltcp` as the TCP/IP stack foundation.

Current work includes:

* packet receive/transmit flow
* embedded network device abstraction
* packet pool management
* ARP and ICMP handling experiments
* future Wi-Fi transport integration

---

## 🖥 Display Subsystem

Current display implementation includes:

* ST7789VW SPI LCD driver
* Framebuffer rendering
* RGB565 graphics pipeline
* DMA-accelerated framebuffer flush
* `embedded-graphics` integration

Planned improvements:

* dirty rectangle optimization
* optional LVGL integration
* display abstraction cleanup

---

## 🛠 Build & Run

```bash
cargo build
cargo run
```

---

## 🧪 Hardware

Currently tested on:

* Raspberry Pi Pico 2
* Raspberry Pi Pico 2W
* Waveshare Pico-LCD-1.14

---

## 📚 Notes

This project serves as an experimental platform for exploring:

* RTOS internals
* Cortex-M exception handling
* task scheduling
* embedded synchronization primitives
* low-level driver development
* embedded networking
* Wi-Fi bring-up
* graphics pipelines
* modern embedded development in Rust

The codebase and architecture continue to evolve as additional subsystems are implemented and refined.

---

## 📄About

This repository is part of my ongoing exploration of low-level systems programming and embedded software architecture in Rust.
