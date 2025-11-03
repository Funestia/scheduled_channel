#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SendTimeoutError<T> {
    /// The message could not be sent because the channel is full and the operation timed out.
    ///
    /// If this is a zero-capacity channel, then the error indicates that there was no receiver
    /// available to receive the message and the operation timed out.
    Timeout(T),

    /// The message could not be sent because the channel is disconnected.
    Disconnected(T),

    MutexPoisoningError(T),

}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RecvTimeoutError {

    /// A message could not be received because the channel is empty and the operation timed out.

    ///

    /// If this is a zero-capacity channel, then the error indicates that there was no sender

    /// available to send a message and the operation timed out.

    Timeout,


    /// The message could not be received because the channel is empty and disconnected.

    Disconnected,
    MutexPoisoningError,

}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvError {

    /// A message could not be received because the channel is empty.

    ///

    /// If this is a zero-capacity channel, then the error indicates that there was no sender

    /// available to send a message at the time.

    Empty,


    /// The message could not be received because the channel is empty and disconnected.

    Disconnected,

    MutexPoisoningError,
}
