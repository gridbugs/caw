use super::moog_ladder_huovilainen::State as MoogLadderHuovilainenState;
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

impl MoogLadderState for MoogLadderHuovilainenState {
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
        // The amplitude of the signal seems to decrease as resonance increases, so compensate for
        // this here.
        self.process_sample(sample) * (1.0 + (self.resonance() * 4.0))
    }
}
