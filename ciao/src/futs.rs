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

// Await two futures at once, but stop as soon as one of
// them completes. Return enum indicates which future was
// the first to complete, or in case that the compound
// future is being polled after completion, to warn that
// this is the case by returnin None. If both futures
// complete before compound future is polled again, the
// return value will indicate that the first future is
// the "winner", and second future won't get pulled to its
// completion at all.
pub(crate) struct Select<A, B>
where
    A: Future<Output = ()> + Unpin,
    B: Future<Output = ()> + Unpin,
{
    a: A,
    b: B,
    done: bool,
}

// We impose a simplifying constraint, just to avoid unsafe code.
impl<A, B> Unpin for Select<A, B>
where
    A: Future<Output = ()> + Unpin,
    B: Future<Output = ()> + Unpin,
{
}

pub(crate) enum FirstSecond {
    First,
    Second,
}

impl<A, B> Future for Select<A, B>
where
    A: Future<Output = ()> + Unpin,
    B: Future<Output = ()> + Unpin,
{
    type Output = Option<FirstSecond>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // We consume the pinned pointer to self to obtain a mutable ref.
        // This is all safe, because we are guaranteed that both futures
        // we're selecting between implement Unpin, so pinning does not
        // make a difference. For the same reason, we can use the safe
        // methods to construct pinned pointers to struct members. If
        // the members wouldn't implement Pin, then it would be up to
        // us to uphold the pinning guarantees or bear the consequences
        // of the ensuing UB.
        let this = self.get_mut();
        let first = core::pin::Pin::new(&mut this.a);
        let second = core::pin::Pin::new(&mut this.b);

        // While Select as Future is safe to poll after it returned Ready,
        // we have no such guarantees for the two inner futures, so we
        // must make sure to not poll them again afterwards. Alternative to
        // returning None, we could remember which future completed and
        // just keep returning that, although that seems more likely to
        // lead to some logic bugs later down the road.
        if this.done == true {
            core::task::Poll::Ready(None)
        } else if let core::task::Poll::Ready(()) = first.poll(cx) {
            this.done = true;
            core::task::Poll::Ready(Some(FirstSecond::First))
        } else if let core::task::Poll::Ready(()) = second.poll(cx) {
            this.done = true;
            core::task::Poll::Ready(Some(FirstSecond::Second))
        } else {
            core::task::Poll::Pending
        }
    }
}

// Await two futures at once, and keep polling until both of them complete.
// For purpose of illustration, we don't make any assumption about the two
// futures implementing Unpin.
pub(crate) struct Join<A, B>
where
    A: Future<Output = ()>,
    B: Future<Output = ()>,
{
    a: A,
    b: B,
    donea: bool,
    doneb: bool,
}

impl<A, B> Future for Join<A, B>
where
    A: Future<Output = ()>,
    B: Future<Output = ()>,
{
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // We consume the pinned pointer to self to obtain a mutable ref.
        // Since we don't assume inner futures are Unpin, we have to use
        // the unsafe methods. But otherwise the mechanism doesn't differ
        // much from what we've done in Future impl for Select. We obtain
        // a mutable reference by consuming the Pinned reference, and
        // with it we can obtain mutable references to inner futures that
        // we then pin and use for polling the futures.
        //
        // Safety: we don't move the futures anywhere or invalidate them
        // by any other means, so the pinning guarantees are upheld from
        // our side.
        let this = unsafe { self.get_unchecked_mut() };
        let first = unsafe { core::pin::Pin::new_unchecked(&mut this.a) };
        let second = unsafe { core::pin::Pin::new_unchecked(&mut this.b) };

        if !this.donea && core::task::Poll::Ready(()) == first.poll(cx) {
            this.donea = true;
        }

        if !this.doneb && core::task::Poll::Ready(()) == second.poll(cx) {
            this.doneb = true;
        }

        if this.donea && this.doneb {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}

pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future<Output = ()> + Unpin,
    B: Future<Output = ()> + Unpin,
{
    Select { a, b, done: false }
}

pub fn join<A, B>(a: A, b: B) -> Join<A, B>
where
    A: Future<Output = ()>,
    B: Future<Output = ()>,
{
    Join {
        a,
        b,
        donea: false,
        doneb: false,
    }
}
