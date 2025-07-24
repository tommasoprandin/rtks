use heapless::Vec;
use core::ops::AddAssign;

const REQUEST_BUFFER_RANGE: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RequestBufferIndex(usize);

impl RequestBufferIndex {
    fn first() -> Self {
        RequestBufferIndex(0)
    }

    fn last() -> Self {
        RequestBufferIndex(REQUEST_BUFFER_RANGE - 1)
    }
}

// Override += operator
impl AddAssign<usize> for RequestBufferIndex {
    fn add_assign(&mut self, rhs: usize) {
        self.0 = (self.0 + rhs) % REQUEST_BUFFER_RANGE;
    }
}

pub struct RequestBuffer {
    my_request_buffer: Vec<u32, REQUEST_BUFFER_RANGE>,
    insert_index: RequestBufferIndex,
    extract_index: RequestBufferIndex,
    current_size: usize,
    barrier: bool, // TODO: Signal(bool)
}

impl RequestBuffer {
    pub fn new() -> Self {
        RequestBuffer {
            my_request_buffer: Vec::new(),
            insert_index: RequestBufferIndex::first(),
            extract_index: RequestBufferIndex::first(),
            current_size: 0,
            barrier: false,
        }
    }

    pub fn deposit(&mut self, activation_parameter: u32) -> bool {
        if self.current_size < RequestBufferIndex::last().0 {
            let _ = self.my_request_buffer.push(activation_parameter); // TODO: Handle possible error
            self.insert_index += 1;
            self.current_size += 1;
            self.barrier = true;
            return true;
        } else {
            return false;
        }
    }

    pub fn extract(&mut self) -> u32 {
        // TODO: handle barrier as entry guard
        if !self.barrier {
            return 1;
        } else {
            self.extract_index += 1;
            self.current_size -= 1;
            self.barrier = false;
            return self.my_request_buffer.pop().unwrap();
        }
    }
}