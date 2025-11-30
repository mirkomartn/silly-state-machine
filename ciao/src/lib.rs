#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        ()
    }
}

// Initialize state machine. Must be called at least once, before
// trying to progress with the state machine execution.
#[unsafe(no_mangle)]
pub extern "C" fn init() {}

// Single-step the state machine. This might cause a state
// transition, although the state might remain the same. The
// execution is expected to be loosely bounded -- that is,
// there will be no indeterminantly blocking calls, but a step
// might perform expensive computations varying the amount of
// time this function takes to return.
#[unsafe(no_mangle)]
pub extern "C" fn step() {}

