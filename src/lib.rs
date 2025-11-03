//!A scheduled channel.
//!
//!A scheduled channel works like a normal mpmc channel but it has the option
//!to supply messages with a timestamp they should be read at.
//!
//!## Example
//!```
//!use scheduled_channel::bounded;
//!use std::time::{Instant, Duration};
//!
//!let (sender, receiver) = bounded(1000);
//!
//!sender.send(0, None).unwrap();
//!sender.send(5, Some(Instant::now() + Duration::from_secs(5))).unwrap();
//!sender.send(4, Some(Instant::now() + Duration::from_secs(4))).unwrap();
//!sender.send(6, Some(Instant::now() + Duration::from_secs(6))).unwrap();
//!sender.send(3, Some(Instant::now() + Duration::from_secs(3))).unwrap();
//!sender.send(2, Some(Instant::now() + Duration::from_secs(2))).unwrap();
//!sender.send(1, None).unwrap();
//!
//!for i in 0..=6 {
//!    assert_eq!(receiver.recv().unwrap(), i);
//!}
//!```

#![feature(generic_atomic)]
#![feature(lock_value_accessors)]
#![feature(thread_sleep_until)]
#![feature(mpmc_channel)]

use std::{panic::{RefUnwindSafe, UnwindSafe}, time::{Duration, Instant}};

use crate::{array::Channel, err::{RecvTimeoutError, SendTimeoutError, TryRecvError}};
mod waker;
mod context;
mod array;
mod select;
mod counter;
mod err;

pub struct Sender<T> {
    inner: crate::counter::Sender<Channel<T>>
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

impl<T> Sender<T> {

/// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// A successful send occurs when it is determined that the other end of
    /// the channel has not hung up already. An unsuccessful send would be one
    /// where the corresponding receiver has already been deallocated. Note
    /// that a return value of [`Err`] means that the data will never be
    /// received, but a return value of [`Ok`] does *not* mean that the data
    /// will be received. It is possible for the corresponding receiver to
    /// hang up immediately after this function returns [`Ok`]. However, if
    /// the channel is zero-capacity, it acts as a rendezvous channel and a
    /// return value of [`Ok`] means that the data has been received.
    ///
    /// If the channel is full and not disconnected, this call will block until
    /// the send operation can proceed. If the channel becomes disconnected,
    /// this call will wake up and return an error. The returned error contains
    /// the original message.
    ///
    /// If called on a zero-capacity channel, this method will wait for a receive
    /// operation to appear on the other side of the channel.
    ///
    /// If a time is specified, a reader will be able to receive the message after the deadline.
    /// messages will arrive in the order they are sent in except when a timestamp is specified.
    /// This means a message with a timestamp won't prevent messages that are sent after it from
    /// arriving before it's deadline is met
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// fn main() {
    ///     let (tx, rx) = scheduled_channel::bounded(1000);
    ///    
    ///     // the messages will arrive in the order 1,2,3,4
    ///     tx.send(3, Some(Instant::now() + Duration::from_secs(3))).unwrap();
    ///     tx.send(2, Some(Instant::now() + Duration::from_secs(2))).unwrap();
    ///     tx.send(4, Some(Instant::now() + Duration::from_secs(4))).unwrap();
    ///     tx.send(1, None).unwrap();
    ///     
    ///     assert_eq!(rx.recv().unwrap(), 1);
    ///     assert_eq!(rx.recv().unwrap(), 2);
    ///     assert_eq!(rx.recv().unwrap(), 3);
    ///     assert_eq!(rx.recv().unwrap(), 4);
    ///    
    ///     // This send will fail because the receiver is gone
    ///     drop(rx);
    ///     assert!(tx.send(1, None).is_err());
    /// }
    /// ```
    pub fn send(&self, msg: T, time: Option<Instant>) -> Result<(), SendTimeoutError<T>> {
        self.inner.send(msg, time, None)
    }
    
/// Waits for a message to be sent into the channel, but only for a limited time.
    ///
    /// If the channel is full and not disconnected, this call will block until the send operation
    /// can proceed or the operation times out. If the channel becomes disconnected, this call will
    /// wake up and return an error. The returned error contains the original message.
    ///
    /// If called on a zero-capacity channel, this method will wait for a receive operation to
    /// appear on the other side of the channel.
    /// 
    /// read about `send` for more details
    /// # Examples
    ///
    /// ```
    /// use scheduled_channel::bounded;
    /// use std::time::{Duration, Instant};
    ///
    /// let (tx, rx) = bounded(1000);
    ///
    /// tx.send_timeout(1, None, Duration::from_millis(400)).unwrap();
    /// ```
    pub fn send_timeout(&self, msg: T, time: Option<Instant>, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
        match Instant::now().checked_add(timeout) {
            Some(deadline) => self.send_deadline(msg, time, deadline),
            None => self.send(msg, time)
        }
    }

    /// Waits for a message to be sent into the channel, but only until a given deadline.
    ///
    /// If the channel is full and not disconnected, this call will block until the send operation
    /// can proceed or the operation times out. If the channel becomes disconnected, this call will
    /// wake up and return an error. The returned error contains the original message.
    ///
    /// If called on a zero-capacity channel, this method will wait for a receive operation to
    /// appear on the other side of the channel.
    ///
    ///read about `send` for more details
    /// # Examples
    ///
    /// ```
    /// use scheduled_channel::bounded;
    /// use std::time::{Duration, Instant};
    ///
    /// let (tx, rx) = bounded(1000);
    ///
    /// let t = Instant::now() + Duration::from_millis(400);
    /// tx.send_deadline(1, None, t).unwrap();
    /// ```
    pub fn send_deadline(&self, msg: T, time: Option<Instant>, deadline: Instant) -> Result<(), SendTimeoutError<T>>{
        self.inner.send(msg, time, Some(deadline))
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }
    pub fn same_channel(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.release(|c|c.disconnect());
        }
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self{inner: self.inner.acquire()}
    }
}
pub struct Receiver<T> {
    inner: crate::counter::Receiver<Channel<T>>
}

impl<T> UnwindSafe for Receiver<T> {}
impl<T> RefUnwindSafe for Receiver<T> {}

impl<T> Receiver<T> {

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.inner.try_recv()
    }
    pub fn recv(&self) -> Result<T, RecvTimeoutError> {
        self.inner.recv(None)
    }
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        match Instant::now().checked_add(timeout) {
            Some(deadline) => self.recv_deadline(deadline),
            None => self.recv()
        }
    }
    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
        self.inner.recv(Some(deadline))
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }
    pub fn same_channel(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        unsafe {
            self.inner.release(|c|c.disconnect());
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {inner: self.inner.acquire()}
    }
}

/// This creates a new scheduled channel
pub fn bounded<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = counter::new(Channel::with_capacity(cap));
        let s = Sender {
            inner: s
        };
        let r = Receiver {
            inner: r
        };
        (s, r)
}

#[cfg(test)]
mod tests {
    use std::{thread::{self, sleep}, time::{Duration, Instant}};

    use crate::bounded;


    #[test]
    fn it_works() {
        let (sender, receiver) = bounded(100);
        let sender2 = sender.clone();
        sender.send(4, Some(Instant::now() + Duration::from_millis(1500))).unwrap();
        thread::spawn(move|| {
            sender2.send(3, Some(Instant::now() + Duration::from_millis(1000))).unwrap();
        });
        sender.send(2, Some(Instant::now() + Duration::from_millis(500))).unwrap();
        sender.send(0, None).unwrap();
        sender.send(1, None).unwrap();
        sleep(Duration::from_millis(700));
        assert_eq!(receiver.recv().unwrap(), 0);
        println!("0 works");
        assert_eq!(receiver.recv().unwrap(), 1);
        println!("1 works");
        assert_eq!(receiver.recv().unwrap(), 2);
        println!("2 works");
        assert_eq!(receiver.recv().unwrap(), 3);
        println!("3 works");
        assert_eq!(receiver.recv().unwrap(), 4);
        println!("4 works");
    }
}
