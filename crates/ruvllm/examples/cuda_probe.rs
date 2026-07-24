//! CUDA device smoke test for the ruvllm inference-cuda backend.
//!
//! Run:  cargo run -p ruvllm --release --example cuda_probe --features inference-cuda,fused-act
//!
//! Opens CUDA device 0 via candle, prints the detected device, and exits 0 on
//! success / 1 if no CUDA device is reachable. Used as the AC3 gate for the
//! gha-runner CUDA-agent phase.

use candle_core::Device;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Device::new_cuda(0) {
        Ok(dev) => {
            println!("CUDA device 0 opened: {dev:?}");
            // Exercise the device with a tiny allocation so we prove the
            // driver + PTX JIT path, not just handle creation.
            let t = candle_core::Tensor::zeros((2, 3), candle_core::DType::F32, &dev)?;
            println!("allocated {:?} on {:?}", t.shape(), t.device());
            Ok(())
        }
        Err(e) => {
            eprintln!("no CUDA device reachable: {e}");
            std::process::exit(1);
        }
    }
}
