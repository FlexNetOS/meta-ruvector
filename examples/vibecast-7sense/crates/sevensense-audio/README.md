# sevensense-audio

[![Crate](https://img.shields.io/badge/crates.io-sevensense--audio-orange.svg)](https://crates.io/crates/sevensense-audio)
[![Docs](https://img.shields.io/badge/docs-sevensense--audio-blue.svg)](https://docs.rs/sevensense-audio)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

> Audio ingestion and preprocessing pipeline for bioacoustic analysis.

**sevensense-audio** handles audio input for the 7sense platform: decoding files,
converting to mono, resampling to a standard 32 kHz, energy-based segmentation into
call segments, and computing mel spectrograms for downstream neural-network input.

The crate follows Domain-Driven Design with clean architecture:

- **Domain Layer**: Core entities (`Recording`, `CallSegment`, `SignalQuality`) and the `RecordingRepository` trait
- **Application Layer**: `AudioIngestionService` orchestrating ingestion and segmentation
- **Infrastructure Layer**: `SymphoniaFileReader`, `RubatoResampler`, `EnergySegmenter`
- **Spectrogram**: `MelSpectrogram` and `SpectrogramConfig`

## Features

- **Multi-Format Decoding**: WAV, FLAC, MP3, Ogg via [Symphonia](https://crates.io/crates/symphonia) (`SymphoniaFileReader`)
- **Resampling**: Sample-rate conversion to 32 kHz via [Rubato](https://crates.io/crates/rubato) (`RubatoResampler`)
- **Energy-Based Segmentation**: Split recordings into call segments (`EnergySegmenter`)
- **Mel Spectrograms**: Configurable FFT, hop length, and mel bins (`MelSpectrogram`)

## Use Cases

| Use Case | Description | Key API |
|----------|-------------|---------|
| File Ingestion | Decode, mono-ize, resample to 32 kHz | `AudioIngestionService::ingest_file()` |
| Segmentation | Split a recording into call segments | `AudioIngestionService::segment_recording()` |
| Spectrogram | Convert samples to a mel spectrogram | `MelSpectrogram::compute()` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sevensense-audio = "0.1"
```

## Quick Start

`AudioIngestionService` is composed from three infrastructure components, each
wrapped in an `Arc` and passed to `new`:

```rust,no_run
use sevensense_audio::application::AudioIngestionService;
use sevensense_audio::infrastructure::{SymphoniaFileReader, RubatoResampler, EnergySegmenter};
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the infrastructure components
    let reader = Arc::new(SymphoniaFileReader::new());
    let resampler = Arc::new(RubatoResampler::new(32_000)?); // target sample rate
    let segmenter = Arc::new(EnergySegmenter::default());

    // Assemble the service
    let service = AudioIngestionService::new(reader, resampler, segmenter);

    // Ingest an audio file -> Recording
    let mut recording = service.ingest_file(Path::new("birdsong.wav")).await?;
    println!("Duration: {} ms", recording.duration_ms());

    // Segment the recording into call segments
    let segments = service.segment_recording(&mut recording).await?;
    println!("Found {} call segments", segments.len());

    Ok(())
}
```

The crate also exposes the constants `TARGET_SAMPLE_RATE` (32 kHz) and
`STANDARD_SEGMENT_DURATION_MS` (5 000 ms).

---

<details>
<summary><b>Tutorial: Ingesting Audio Files</b></summary>

`ingest_file` reads the file, converts it to mono, and resamples it to 32 kHz,
returning a `Recording` whose samples are loaded and ready for segmentation.

```rust,no_run
use sevensense_audio::application::AudioIngestionService;
use sevensense_audio::infrastructure::{SymphoniaFileReader, RubatoResampler, EnergySegmenter};
use std::path::Path;
use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let service = AudioIngestionService::new(
    Arc::new(SymphoniaFileReader::new()),
    Arc::new(RubatoResampler::new(32_000)?),
    Arc::new(EnergySegmenter::default()),
);

let recording = service.ingest_file(Path::new("recording.mp3")).await?;

// Recording fields and methods
println!("Duration: {} ms", recording.duration_ms());
println!("Segments so far: {}", recording.segment_count());
println!("Processed: {}", recording.is_processed());
# Ok(())
# }
```

`SymphoniaFileReader` implements the `AudioFileReader` trait and reports which
extensions it supports via `supports_extension`.

</details>

<details>
<summary><b>Tutorial: Energy-Based Segmentation</b></summary>

`EnergySegmenter` implements the `AudioSegmenter` trait and detects call segments
based on signal energy. It is driven through the service:

```rust,no_run
use sevensense_audio::application::AudioIngestionService;
use sevensense_audio::infrastructure::{SymphoniaFileReader, RubatoResampler, EnergySegmenter};
use std::path::Path;
use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let service = AudioIngestionService::new(
    Arc::new(SymphoniaFileReader::new()),
    Arc::new(RubatoResampler::new(32_000)?),
    Arc::new(EnergySegmenter::new()),
);

let mut recording = service.ingest_file(Path::new("long_recording.wav")).await?;

// Returns the detected CallSegments (also stored on the recording)
let segments = service.segment_recording(&mut recording).await?;

for seg in &segments {
    println!("Segment: {} ms, quality {:?}", seg.duration_ms(), seg.signal_quality);
}

// Filter to high-quality segments only
let good = recording.high_quality_segments();
println!("{} high-quality segments", good.len());
# Ok(())
# }
```

</details>

<details>
<summary><b>Tutorial: Mel Spectrograms</b></summary>

`MelSpectrogram::compute` takes raw `f32` samples plus a `SpectrogramConfig`
and returns the spectrogram; `shape()` reports its dimensions.

```rust,no_run
use sevensense_audio::{MelSpectrogram, SpectrogramConfig};

# fn example(samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
// Default config: 128 mel bins, 2048 FFT, 512 hop, 32 kHz
let mel = MelSpectrogram::compute(samples, SpectrogramConfig::default())?;

let (frames, bins) = mel.shape();
println!("Mel spectrogram: {frames} frames x {bins} bins");
# Ok(())
# }
```

Custom configuration:

```rust,no_run
use sevensense_audio::SpectrogramConfig;

let config = SpectrogramConfig {
    n_mels: 128,        // Number of mel frequency bands
    n_fft: 2048,        // FFT window size
    hop_length: 512,    // Hop between frames
    sample_rate: 32_000,
    f_min: 0.0,         // Minimum frequency (Hz)
    f_max: 16_000.0,    // Maximum frequency (Hz), Nyquist for 32 kHz
    log_scale: true,    // Apply log scaling
    ref_db: 1.0,        // Reference value for dB conversion
    min_value: 1e-10,   // Floor to avoid log(0)
};
```

</details>

---

## Configuration

### `SpectrogramConfig` Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `n_mels` | 128 | Number of mel frequency bands |
| `n_fft` | 2048 | FFT window size in samples |
| `hop_length` | 512 | Samples between frames |
| `sample_rate` | 32 000 | Input sample rate (Hz) |
| `f_min` | 0.0 | Minimum frequency (Hz) |
| `f_max` | 16 000.0 | Maximum frequency (Hz) |
| `log_scale` | true | Apply log scaling |
| `ref_db` | 1.0 | Reference value for dB conversion |
| `min_value` | 1e-10 | Floor to avoid log(0) |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `TARGET_SAMPLE_RATE` | 32 000 | Standard processing sample rate |
| `STANDARD_SEGMENT_DURATION_MS` | 5 000 | Standard segment duration |

## Public API

Re-exported at the crate root (`sevensense_audio::`):

| Category | Items |
|----------|-------|
| Service | `AudioIngestionService` |
| Entities | `Recording`, `CallSegment`, `SignalQuality` |
| Repository | `RecordingRepository` |
| Spectrogram | `MelSpectrogram`, `SpectrogramConfig` |
| Errors | `AudioError`, `AudioResult` |

Infrastructure types live under `sevensense_audio::infrastructure`:
`SymphoniaFileReader`, `RubatoResampler`, `EnergySegmenter`, and the traits
`AudioFileReader`, `AudioResampler`, `AudioSegmenter`.

## Planned / Not Yet Implemented

The following capabilities are **not** part of the current public API. They are
listed as roadmap items only — do not depend on them yet:

- **Streaming input**: real-time microphone / line-in capture
- **Audio augmentation**: time stretch, pitch shift, noise injection
- **Voice-activity-based (VAD) segmentation** beyond the energy segmenter
- **Spectrogram visualization helpers**

## Links

- **Homepage**: [ruv.io](https://ruv.io)
- **Repository**: [github.com/FlexNetOS/ruvector](https://github.com/FlexNetOS/ruvector)
- **Crates.io**: [crates.io/crates/sevensense-audio](https://crates.io/crates/sevensense-audio)
- **Documentation**: [docs.rs/sevensense-audio](https://docs.rs/sevensense-audio)

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

*Part of the [7sense Bioacoustic Intelligence Platform](https://ruv.io) by rUv*
