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
