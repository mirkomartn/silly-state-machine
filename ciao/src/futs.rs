// Enum that acts as a waiting future, initialized with N and
// lets you poll it N times, before being driven to completion.
pub(crate) enum Wait<const N: u8> {
    Wait(u8),
    Done,
}

// A Wait struct can be moved freely in any state.
impl<const N: u8> Unpin for Wait<N> {}

impl<const N: u8> Future for Wait<N> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // Because Wait implements Unpin, we can use a safe method to obtain a mut ref.
        let s = self.get_mut();
        // Check internal state and act accordingly.
        match *s {
            Wait::Wait(count) => {
                if count < N {
                    *s = Wait::Wait(count + 1);
                    core::task::Poll::Pending
                } else {
                    *s = Wait::Done;
                    core::task::Poll::Ready(())
                }
            }
            Wait::Done => core::task::Poll::Ready(()),
        }
    }
}

// Normally, you could pass or share ownership of peripherals between C and Rust, or
// setup callback that can be called from irq handlers, share mutable statics, etc. But
// for the illustrative purposes we can just expose two functions that our state machine
// can use to querry external state.
//
// Safety: as safe as external C functions can get, no pointer passing, etc., although,
// we don't know what they reallly do, so tread carefully.
unsafe extern "C" {
    fn msg_received() -> bool;
    fn button_pressed() -> bool;
}

pub(crate) struct MessageReceived();

// Strictly speaking, this is not necessary, because we *don't care* about the
// pinned pointer passed to the poll method, but it does make a difference when
// composing futures, as some methods might require Unpin.
impl Unpin for MessageReceived {}

impl Future for MessageReceived {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // Safety: not safe, FFI call into C part.
        if unsafe { msg_received() } == true {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}

pub(crate) struct ButtonPressed();

// See comment to impl Unpin for MessageReceived.
impl Unpin for ButtonPressed {}

impl Future for ButtonPressed {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // Safety: not safe, FFI call into C part.
        if unsafe { button_pressed() } == true {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}
