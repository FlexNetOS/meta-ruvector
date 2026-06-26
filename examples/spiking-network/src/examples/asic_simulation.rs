//! ASIC simulation demo.
//!
//! Routes spike packets across a modeled multi-core ASIC fabric
//! ([`AsicRouter`]) and reports hop count and routing energy, illustrating how
//! the event-driven, sparse representation keeps inter-core traffic — and thus
//! power — low.

use spiking_network::router::{AsicRouter, RouterConfig, SpikePacket};

fn main() {
    let config = RouterConfig {
        num_cores: 8,
        buffer_size: 64,
        hop_energy_pj: 2.0,
    };
    let mut router = AsicRouter::new(config).expect("valid router config");

    println!(
        "ASIC fabric: {} cores, {}-bit spike packets",
        config.num_cores,
        SpikePacket::bit_size()
    );

    // Simulate a wave of spikes fanning out from a source cluster.
    let mut delivered = 0_u32;
    for source in 0..4_u32 {
        for target in 0..config.num_cores as u32 {
            let packet = SpikePacket::new(source, target, source as f32);
            match router.route(packet) {
                Ok(_) => delivered += 1,
                Err(e) => println!("  dropped packet {source}->{target}: {e}"),
            }
        }
    }

    println!("Delivered {delivered} packets");
    println!("Total inter-core hops: {}", router.total_hops());
    println!(
        "Routing energy: {:.1} pJ (avg {:.2} pJ/packet)",
        router.routing_energy_pj(),
        router.routing_energy_pj() / delivered.max(1) as f32
    );
}
