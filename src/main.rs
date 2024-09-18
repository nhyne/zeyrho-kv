use std::collections::VecDeque;
use std::fmt;
use std::fmt::Formatter;

fn main() {

    let mut firstQueue = SimpleQueue{queue: VecDeque::new()};

    firstQueue.enqueue(3);
    println!("first size {}",firstQueue.size());
    firstQueue.enqueue(4);
    println!("second size {}",firstQueue.size());

    let v = firstQueue.dequeue();
    println!("firs pop {:?}",v);

    println!("third size {}",firstQueue.size());

}

struct SimpleQueue {
    queue: VecDeque<u32>
}

impl Queue for SimpleQueue {
    fn enqueue(&mut self, num: u32) -> () {
        self.queue.push_back(num)
    }

    fn dequeue(&mut self) -> Option<u32> {
        self.queue.pop_front()
    }

    fn size(&self) -> u32 {
        VecDeque::len(&self.queue) as u32
    }
}

trait Queue {
    fn enqueue(&mut self, num: u32) -> ();
    fn dequeue(&mut self) -> Option<u32>;
    fn size(&self) -> u32;
}

