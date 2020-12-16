//use common::IO_BASE;
use volatile::Volatile;

/// The base address for the ARM system timer registers.
//const TIMER_REG_BASE: usize = IO_BASE + 0x3000;

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    CS: u32,
    CLO: u32,
    CHI: u32,
    COMPARE: [u32; 4]
}

/// The Raspberry Pi ARM system timer.
pub struct Timer {
    registers: &'static mut Registers
}

impl Timer {
    /// Returns a new instance of `Timer`.
    pub fn new(base: usize) -> Timer {
        Timer {
            registers: unsafe { &mut *(base as *mut Registers) },
        }
    }

    /// Reads the system timer's counter and returns the 64-bit counter value.
    /// The returned value is the number of elapsed microseconds.
    pub fn read(&self) -> u64 {
        let low = Volatile::new_read_only(&self.registers.CLO).read();
        let high = Volatile::new_read_only(&self.registers.CHI).read();
        ((high as u64) << 32) | (low as u64)
    }

    /// Sets up a match in timer 1 to occur `us` microseconds from now. If
    /// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
    /// interrupt will be issued in `us` microseconds.
    pub fn tick_in(&mut self, us: u32) {
        let current_low = Volatile::new_read_only(&mut self.registers.CLO).read();
        let compare = current_low.wrapping_add(us);
        Volatile::new_write_only(&mut self.registers.COMPARE[1]).write(compare); // timer 1
        Volatile::new(&mut self.registers.CS)
            .update(|x| *x |= 0b0010); // clear timer 1 interrupt
    }
}

///// Returns the current time in microseconds.
//pub fn current_time() -> u64 {
//    Timer::new().read()
//}
//
///// Spins until `us` microseconds have passed.
//pub fn spin_sleep_us(us: u64) {
//    let old = current_time();
//    loop {
//        let new = current_time();
//        if old + us <= new {
//            break;
//        }
//    }
//}
//
///// Spins until `ms` milliseconds have passed.
//pub fn spin_sleep_ms(ms: u64) {
//    spin_sleep_us(ms * 1000);
//}
//
///// Sets up a match in timer 1 to occur `us` microseconds from now. If
///// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
///// interrupt will be issued in `us` microseconds.
//pub fn tick_in(us: u32) {
//    Timer::new().tick_in(us)
//}
