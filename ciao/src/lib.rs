#![cfg_attr(not(feature = "std"), no_std)]
#![feature(type_alias_impl_trait)]

use core::{
    future::Future,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};
use spin::mutex::SpinMutex;

mod futs;
use futs::*;

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        ()
    }
}

static TASK: SpinMutex<Option<FutAlias>> = SpinMutex::new(None);

type FutAlias = impl Future + Send;

#[define_opaque(FutAlias)]
fn alias_fn() -> FutAlias {
    get_mail_or_gtfo()
}

// A silly feedback mechanism, to let C know a message was received *under*
// conditions checked by this state machine.
unsafe extern "C" {
    fn got_mail();
}

async fn get_mail_or_gtfo() -> () {
    // Simple state machine that checks if there were at least 9 ticks since
    // the last received message and a button was pressed, if those two
    // conditions are met, then we stop listening for messages.
    loop {
        match select(
            join(ButtonPressed {}, Wait::Wait::<8>(0)),
            MessageReceived {},
        )
        .await
        {
            Some(FirstSecond::First) => break,
            // Safety: unsafe, it's a ffi C function call.
            Some(FirstSecond::Second) => unsafe {
                got_mail();
            },
            None => (),
        };
    }
}

// Initialize state machine. Must be called at least once, before trying to progress with
// the state machine execution.
#[unsafe(no_mangle)]
pub extern "C" fn init() {
    if let Some(mut task) = TASK.try_lock() {
        if task.is_none() {
            *task = Some(alias_fn());
        }
    }
}

// Return a dummy RawWaker that can be used to construct a Waker that does *nothing*.
// This is actually pulled from behind the veil from `futures_task`, see
// https://docs.rs/futures-task/0.3.31/src/futures_task/noop_waker.rs.html.
const fn noop_raw_waker() -> RawWaker {
    fn noop(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }

    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

    RawWaker::new(core::ptr::null(), &VTABLE)
}

// Single-step the state machine. This might cause a state transition, although the state might
// remain the same. The execution is expected to be loosely bounded -- that is, there will be no
// indeterminantly blocking calls, but a step might perform expensive computations varying the
// amount of time this function takes to return.
//
// Actually polls the future that was previously stored in the TASK static. We try to obtain it
// by calling try_lock(), which should make this function reentrant (and also guard against any
// data races with the `init` function).
//
// Returns a boolean -- if true is returned then a terminal state has been reached. The function
// can be called afterwards freely and safely, but no computation will be performed with the
// state machine, until it is re-initialized again.
#[unsafe(no_mangle)]
pub extern "C" fn step() -> bool {
    if let Some(mut task) = TASK.try_lock() {
        if let Some(mut fut) = (*task).take() {
            // Safety: We're constructing Waker with a const fn that we know is valid.
            let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
            let mut cx = Context::from_waker(&waker);

            // Safety: we don't move or modify the future with the obtained mut ref,
            // unless reaching terminal state in which case it gets dropped and so
            // its lifetime ends.
            match unsafe { core::pin::Pin::new_unchecked(&mut fut).poll(&mut cx) } {
                // Terminal state reached, don't re-store future back into the static.
                core::task::Poll::Ready(_) => {
                    return true;
                }
                core::task::Poll::Pending => *task = Some(fut),
            }
        }
    }

    return false;
}
