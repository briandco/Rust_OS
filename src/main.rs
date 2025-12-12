#![no_std]
#![no_main]

use core::panic::PanicInfo;
use riscv_rt::entry;

mod kernel;
use kernel::{Priority,TaskState,TickType};

const UART_BASE: usize = 0x10000000;

fn uart_putc(c: u8) {
    unsafe {
        core::ptr::write_volatile(UART_BASE as *mut u8, c);
    }
}

fn uart_puts(s: &str) {
    for b in s.bytes() {
        uart_putc(b);
    }
}

#[entry]
fn main() -> ! {
    uart_puts("Hello from Rust RTOS!\r\n");
    uart_puts("Running on QEMU virt!\r\n");
    
    // Test our new types
    uart_puts("Testing kernel types...\r\n");
    
    let tick = TickType::new(100);
    uart_puts("TickType created\r\n");
    
    let state = TaskState::Ready;
    uart_puts("TaskState created\r\n");
    
    uart_puts("All types working!\r\n");
    
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart_puts("\r\nPANIC!\r\n");
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}
