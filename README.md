# A scheduled channel.

A scheduled channel works like a normal mpmc channel but it has the option
to supply messages with a timestamp they should be read at.

## Example
```
use scheduled_channel::bounded;
use std::time::{Instant, Duration};

let (sender, receiver) = bounded(1000);

sender.send(0, None).unwrap();
sender.send(5, Some(Instant::now() + Duration::from_secs(5))).unwrap();
sender.send(4, Some(Instant::now() + Duration::from_secs(4))).unwrap();
sender.send(6, Some(Instant::now() + Duration::from_secs(6))).unwrap();
sender.send(3, Some(Instant::now() + Duration::from_secs(3))).unwrap();
sender.send(2, Some(Instant::now() + Duration::from_secs(2))).unwrap();
sender.send(1, None).unwrap();

for i in 0..=6 {
    assert_eq!(receiver.recv().unwrap(), i);
}
```
