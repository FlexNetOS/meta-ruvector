use spiking_network::{STDPConfig, STDPLearning, SpikeEncoder, SpikingNetwork};
fn main() {
    let mut n = SpikingNetwork::new(200);
    let config = STDPConfig::default();
    let l = STDPLearning::new(config);
    let p: Vec<f32> = (0..100)
        .map(|i| f32::exp(-(i as f32 - 50.0).abs() / 20.0))
        .collect();
    let s = SpikeEncoder::rate_encode(&p, 0.1);
    let _ = n.process_with_learning(&s, &l);
    println!("Pattern recognition complete");
}
