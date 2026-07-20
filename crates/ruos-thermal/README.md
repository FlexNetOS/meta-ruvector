# ruos-thermal

Pi 5 thermal supervisor + over/underclock control (ADR-174).

`ruos-thermal` is a thermal observability and clock-control tool for the
Raspberry Pi 5 (and AI HAT+). It walks `/sys/class/thermal/thermal_zone*` for
temperatures and `/sys/devices/system/cpu/cpufreq/policy*` for current/max
frequency, renders a snapshot, and can optionally apply a clock profile. This is
the iter-91 read-only skeleton; the Unix-socket budget protocol and workload
subscribers land in later iterations of the ADR-174 roadmap.

## Build

```bash
cargo build --release
# binary at target/release/ruos-thermal
```

## Usage

```bash
ruos-thermal                  # default TSV output (one row per zone + per policy)
ruos-thermal --json           # single NDJSON line for jq / log shippers
ruos-thermal --prom           # Prometheus textfile-collector format
ruos-thermal --show-profiles  # list available clock profiles + target MHz
ruos-thermal --version        # print pkg-name + semver
ruos-thermal --help           # print help and exit
```

### Profile control

```bash
# Apply a clock profile (requires the explicit write opt-in)
ruos-thermal --set-profile safe-overclock --allow-cpufreq-write
```

Profiles: `eco`, `default`, `safe-overclock`, `aggressive`, `max`. cpufreq
writes are privileged — without `--allow-cpufreq-write`, `--set-profile` errors
cleanly without touching cpufreq, and applying a profile needs root or
`CAP_SYS_NICE`.

### Options

| Flag | Effect |
|------|--------|
| `--json` | Single NDJSON line (mutually exclusive with `--prom`). |
| `--prom` | Prometheus gauges with HELP/TYPE. |
| `--show-profiles` | List clock profiles; short-circuits before any sensor I/O. |
| `--set-profile <name>` | Apply a clock profile (needs `--allow-cpufreq-write`). |
| `--allow-cpufreq-write` | Operator opt-in for privileged sysfs writes. |
| `-V`, `--version` | Print pkg-name + semver. |
| `-h`, `--help` | Print help. |

Exit codes: `0` snapshot rendered, `1` bad CLI args, `2` sysfs read error.

## Library usage

```rust
use ruos_thermal::ThermalSensor;

fn main() -> std::io::Result<()> {
    let sensor = ThermalSensor::system();
    let snapshot = sensor.read()?;
    for cpu in &snapshot.cpu_temps_celsius {
        println!("zone {} = {:.1}°C", cpu.zone, cpu.celsius);
    }
    for policy in &snapshot.cpu_policies {
        println!("policy {} cur={} max={}", policy.id, policy.cur_hz, policy.max_hz);
    }
    Ok(())
}
```

## Public API

- `ThermalSensor` — sysfs reader (`ThermalSensor::system()`, `.read()`, `.apply_profile()`)
- `ClockProfile` — clock profile enum (`Eco`, `Default`, `SafeOverclock`, `Aggressive`, `Max`)
- `CpuTemp`, `CpuPolicy` — snapshot data types
