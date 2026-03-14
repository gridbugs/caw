use super::moog_ladder_oberheim::State as MoogLadderOberheimState;

pub trait MoogLadderState {
    fn new() -> Self;
    fn update_params(
        &mut self,
        sample_rate_hz: f64,
        cutoff_hz: f64,
        resonance: f64,
    );
    fn process_sample(&mut self, sample: f64) -> f64;
}

impl MoogLadderState for MoogLadderOberheimState {
    fn new() -> Self {
        Self::new()
    }

    fn update_params(
        &mut self,
        sample_rate_hz: f64,
        cutoff_hz: f64,
        resonance: f64,
    ) {
        self.update_params(sample_rate_hz, cutoff_hz, resonance);
    }

    fn process_sample(&mut self, sample: f64) -> f64 {
        self.process_sample(sample)
    }
}
