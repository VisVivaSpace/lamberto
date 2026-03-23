use std::path::PathBuf;
use std::process;

use clap::Parser;

use lamberto::{config, output, scan};

#[derive(Parser)]
#[command(name = "lamberto", about = "Interplanetary trajectory scanner", version)]
struct Cli {
    /// Path to the sweep configuration YAML file
    config: String,

    /// Output directory for CSV and summary files (default: current directory)
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let config = match config::load_config(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            process::exit(1);
        }
    };

    println!("Loaded {} sweep(s) from {}", config.sweeps.len(), cli.config);

    // Load ephemeris (embedded + optional extra SPK from config)
    let almanac = match lamberto::load_almanac(config.spk_file.as_deref()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to load ephemeris: {e}");
            process::exit(1);
        }
    };

    // Run each sweep
    let mut all_results = Vec::new();
    for sweep in &config.sweeps {
        match scan::run_sweep(&almanac, sweep) {
            Ok(result) => {
                result.print_report();
                all_results.push(result);
            }
            Err(e) => {
                eprintln!("Sweep '{}' failed: {e}", sweep.name);
                process::exit(1);
            }
        }
    }

    // Write output
    if let Err(e) = output::write_all(&config, &all_results, &cli.output_dir) {
        eprintln!("Failed to write output: {e}");
        process::exit(1);
    }
}
