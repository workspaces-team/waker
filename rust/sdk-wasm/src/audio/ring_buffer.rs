//! Fixed-size f32 circular ring buffer for audio clip accumulation.

/// A fixed-capacity circular buffer of f32 samples.
///
/// Used to maintain a sliding window of the most recent audio samples
/// (e.g. 2 seconds at 16 kHz = 32,000 samples).
pub struct RingBuffer {
    buffer: Vec<f32>,
    write_index: usize,
    filled: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_index: 0,
            filled: 0,
        }
    }

    /// Append samples to the ring buffer, overwriting oldest data when full.
    pub fn append(&mut self, samples: &[f32]) {
        let cap = self.buffer.len();
        for &sample in samples {
            self.buffer[self.write_index] = sample;
            self.write_index = (self.write_index + 1) % cap;
            if self.filled < cap {
                self.filled += 1;
            }
        }
    }

    /// Returns `true` if the buffer has been filled at least once.
    pub fn is_full(&self) -> bool {
        self.filled >= self.buffer.len()
    }

    /// Capacity (total number of samples the buffer holds).
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Copy the contents of the ring buffer into `output` in chronological order.
    /// The buffer must be full; panics otherwise.
    pub fn snapshot(&self, output: &mut [f32]) {
        assert!(
            self.is_full(),
            "ring buffer is not full yet ({}/{})",
            self.filled,
            self.buffer.len()
        );
        let cap = self.buffer.len();
        debug_assert!(output.len() >= cap);
        let first_chunk_len = cap - self.write_index;
        output[..first_chunk_len].copy_from_slice(&self.buffer[self.write_index..]);
        output[first_chunk_len..cap].copy_from_slice(&self.buffer[..self.write_index]);
    }

    /// Reset the buffer to empty.
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
        self.filled = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_and_snapshot() {
        let mut rb = RingBuffer::new(4);
        assert!(!rb.is_full());
        rb.append(&[1.0, 2.0, 3.0, 4.0]);
        assert!(rb.is_full());
        let mut out = vec![0.0; 4];
        rb.snapshot(&mut out);
        assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn wrap_around_preserves_order() {
        let mut rb = RingBuffer::new(4);
        rb.append(&[1.0, 2.0, 3.0, 4.0]);
        rb.append(&[5.0, 6.0]);
        let mut out = vec![0.0; 4];
        rb.snapshot(&mut out);
        assert_eq!(out, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn reset_clears_state() {
        let mut rb = RingBuffer::new(4);
        rb.append(&[1.0, 2.0, 3.0, 4.0]);
        assert!(rb.is_full());
        rb.reset();
        assert!(!rb.is_full());
    }
}
