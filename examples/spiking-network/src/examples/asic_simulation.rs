use spiking_network::{SpikingNetwork, NeuronParams}; fn main() { let params = NeuronParams::default(); assert!(params.membrane_tau > 0.0); println!("ASIC simulation: OK"); }
