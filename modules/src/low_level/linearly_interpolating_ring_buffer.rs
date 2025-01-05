use std::collections::VecDeque;

pub struct LinearlyInterpolatingRingBuffer(VecDeque<f32>);

impl LinearlyInterpolatingRingBuffer {
    pub fn new(initial_size: usize) -> Self {
        let mut vec_deque = VecDeque::with_capacity(initial_size);
        vec_deque.resize(initial_size, 0.);
        Self(vec_deque)
    }

    pub fn query(&self, mut index: f32) -> Option<f32> {
        if index < 0. {
            log::warn!("Unexpected negative index!");
            index = 0.;
        }
        let floor = index.floor();
        let ceil = index.ceil();
        let floor_value = self.0.get(floor as usize).cloned()?;
        if floor == ceil {
            Some(floor_value)
        } else {
            let offset = index - floor;
            let ceil_value = self.0.get(ceil as usize).cloned()?;
            Some((floor_value * (1.0 - offset)) + (ceil_value * offset))
        }
    }

    pub fn query_resizing(&mut self, mut index: f32) -> f32 {
        if index < 0. {
            log::warn!("Unexpected negative index!");
            index = 0.;
        }
        let required_min_len = index.ceil() as usize + 1;
        if required_min_len > self.0.len() {
            self.0.resize(required_min_len, 0.);
        }
        self.query(index).unwrap()
    }

    pub fn insert(&mut self, value: f32) {
        self.0.pop_back();
        self.0.push_front(value);
    }
}
