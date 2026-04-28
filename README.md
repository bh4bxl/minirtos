# miniRTOS (RP2350, Rust)

A small experimental bare-metal runtime / mini RTOS project for the RP2350 (Raspberry Pi Pico 2 / Pico 2W), written in Rust.

This project focuses on understanding and building core embedded system components from scratch, including drivers, interrupt handling, and (upcoming) task scheduling.

---

## 🚧 Status

Work in progress.

### ✅ Implemented

- Basic project bring-up on RP2350 (Pico 2W)
- Driver framework (`driver_manager`)
- GPIO driver
- UART driver (PL011-based, using RP235x PAC)
- Interrupt registration and dispatch
- UART RX interrupt handling
- Simple console abstraction over UART
- SysTick-based system tick
- PendSV/SVC-based context switching prototype
- Static task creation and basic scheduler
- LCD driver for ST7789VW
- Framebuffer-based display path
- `embedded-graphics` DrawTarget integration
- RGB565 rendering
- SPI DMA support for LCD framebuffer flush
- 16-bit SPI DMA pixel transfer optimization

### 🚧 In Progress / Planned

- Ring buffer for UART RX/TX
- Scheduler improvements
- Blocking / wakeup primitives
- Mutex and semaphore refinement
- Software timers
- DMA abstraction cleanup
- Improved driver abstraction
- Better BSP separation
- Display driver cleanup and optional LVGL integration

### ❌ Not Implemented Yet

- Memory allocator / heap
- Dynamic task creation
- Full priority scheduling
- Preemptive scheduling policy refinement
- Full BSP abstraction
- Custom clock driver
- Filesystem / storage
- Networking

---

## 🧠 Design Goals

- Keep the system simple and transparent
- Avoid heavy abstractions in early stages
- Build components step-by-step (driver → interrupt → scheduler → ...)
- Maintain clear separation between:
  - platform-independent logic
  - device drivers
  - MCU / board-specific code

---

## 📁 Project Structure

### Layering Overview

```text
Application (main)
   ↓
sys/ (core system)
   ↓
drivers/ (device drivers)
   ↓
bsp/mcu (RP235x implementation)
   ↓
hardware
```

### 🔧 Drivers

- UART
  - Based on PL011
  - Currently tightly coupled with `rp235x_pac`
  - Future goal:
    - separate PL011 logic from PAC-specific implementation
- GPIO
  - Basic GPIO control using RP235x PAC
- SPI
  - Basic SPI control using RP235x PAC
- LCD
  - Waveshare Pico-LCD-1.14 on SPI1

### ⚙️ Interrupt Handling

- Centralized interrupt registration via `sys::interrupt`
- IRQ handlers registered through a descriptor-based mechanism
- RP235x NVIC interaction implemented `in bsp/mcu/rp235x`

### 🔒 Synchronization

- Simple lock primitives:
  - `IrqSafeNullLock` (IRQ-safe critical sections)

These are intentionally minimal for early-stage bring-up.

### 🖥 Console

- Lightweight console abstraction over UART
- Used for debugging output and shell

### ⏱ Clock
- Currently initialized using HAL for simplicity
- Planned:
  - custom clock driver

## 🚀 Roadmap

### Phase 1

- Driver framework
- UART + GPIO
- Interrupt handling

### Phase 2

- SysTick timer
- SVC/PendSV context switch
- Static task scheduler

### Phase 3

- Display subsystem
- Framebuffer
- embedded-graphics integration
- SPI DMA acceleration

### Phase 4

- Synchronization primitives
- Blocking tasks
- Software timers

### Phase 5

- Memory management
- Portable driver model
- Networking experiments
- Multi-platform support

## 🛠 Build & Run

```bash
cargo build
cargo run
```

## 📌 Notes

- This project is experimental and primarily for learning and exploration
- Code structure and abstractions will evolve over time
- Early design favors clarity over completeness

## About

This project is part of my exploration of low-level systems and embedded development in Rust.
