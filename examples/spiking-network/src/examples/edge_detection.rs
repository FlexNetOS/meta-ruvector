use spiking_network::{SpikeEncoder, SpikingNetwork};
fn main() {
    let mut n = SpikingNetwork::new(100);
    let inputs: Vec<f32> = (0..50).map(|i| f32::sin(i as f32 * 0.2) + 1.0).collect();
    let spikes = SpikeEncoder::rate_encode(&inputs, 0.1);
    let _ = n.process(&spikes);
    println!("Edge detection complete");
}
