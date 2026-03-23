# Lamberto

Interplanetary trajectory scanner -- sweeps departure/arrival date grids and
solves Lambert's problem for pork-chop plots and trajectory catalogs.

## What it does

Lamberto takes a YAML configuration file describing one or more trajectory
sweeps, builds a grid of departure and arrival dates, and solves Lambert's
problem for each grid point using the Gooding method (via the
[gooding-lambert](https://crates.io/crates/gooding-lambert) crate). It
outputs a CSV trajectory catalog per sweep and a YAML summary file suitable
for generating pork-chop plots or feeding into downstream mission-design
tools.

## Installation

```
cargo install lamberto
```

## Usage

Run a sweep with default output in the current directory:

```
lamberto config.yaml
```

Specify an output directory:

```
lamberto config.yaml -o output/
```

## Configuration

Sweeps are defined in a YAML file. Here is an annotated example for an
Earth-to-Mars search:

```yaml
# Path to a NAIF SPK ephemeris file (see Ephemeris section below)
spk_file: "de440s.bsp"

sweeps:
  - name: "Earth-Mars Type I"
    departure_body: "Earth"          # or NAIF ID: 3
    arrival_body: "Mars"             # or NAIF ID: 4
    departure_start: "2026-01-01 00:00:00 TDB"
    departure_end: "2026-12-31 00:00:00 TDB"
    departure_step_days: 5.0         # grid spacing in days
    arrival_start: "2026-06-01 00:00:00 TDB"
    arrival_end: "2027-12-31 00:00:00 TDB"
    arrival_step_days: 5.0
    direction: prograde              # prograde (default) or retrograde
    nrev: 0                          # number of complete revolutions (default: 0)
    target_v_inf_departure: 0.0      # km/s, for best-solution ranking
    target_v_inf_arrival: 0.0        # km/s, for best-solution ranking
```

Bodies can be specified by name (`"Earth"`, `"Mars"`, `"Jupiter"`, etc.) or
by NAIF barycenter ID (1-8). Dates use TDB time scale.

## Ephemeris

Lamberto requires a NAIF SPK ephemeris file for planetary positions. The
JPL DE440s ephemeris is recommended and can be downloaded from the
[NASA/JPL NAIF archive](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/).

## Output

Each sweep produces a CSV file named after the sweep (e.g.,
`Earth-Mars Type I.csv`) with the following columns:

| Column | Description |
|---|---|
| `departure_date` | Departure epoch (TDB) |
| `arrival_date` | Arrival epoch (TDB) |
| `tof_days` | Time of flight in days |
| `transfer_angle_deg` | Transfer angle in degrees |
| `type` | Transfer type (I, II, III, IV, etc.; `-R` suffix for retrograde) |
| `c3_departure_km2s2` | Departure C3 in km^2/s^2 |
| `v_inf_departure_kms` | Departure v-infinity in km/s |
| `v_inf_arrival_kms` | Arrival v-infinity in km/s |

A `summary.yaml` file is also written with grid statistics and the
best-found solutions (closest to the target v-infinity values) for each
sweep.

## Dependencies

- [gooding-lambert](https://crates.io/crates/gooding-lambert) -- Gooding's
  method for solving Lambert's problem
- [anise](https://crates.io/crates/anise) -- NAIF SPK ephemeris loading and
  frame transformations

## License

MIT


