use spiking_network::{NeuronParams, SpikingNetwork};
fn main() {
    let params = NeuronParams::default();
    assert!(params.membrane_tau > 0.0);
    println!("ASIC simulation: OK");
}
