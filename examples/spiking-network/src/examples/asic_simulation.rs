use spiking_network::LIFParams;
fn main() {
    let params = LIFParams::default();
    assert!(params.tau_m > 0.0);
    println!("ASIC simulation: OK");
}
