#![no_std]
#![no_main]

use core::panic::PanicInfo;
use riscv_rt::entry;

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
    loop {
        unsafe { riscv::asm::wfi(); }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        unsafe { riscv::asm::wfi(); }
    }
}
