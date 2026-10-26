//! Prepare a complete source-bound default candidate without contacting a device.

use knx_app::system2_validation_preparation::prepare_simulated_candidate;
use knx_core::IndividualAddress;
use std::{collections::BTreeMap, fs, io::Write, path::Path, process::ExitCode};

pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
fn execute(args: &[String]) -> Result<(), String> {
    let mut values = BTreeMap::new();
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        if !matches!(
            flag.as_str(),
            "--target" | "--profile" | "--product-db" | "--output"
        ) {
            return Err("system2-prepare accepts --target --profile --product-db --output only; it never connects".into());
        }
        let value = args
            .next()
            .filter(|v| !v.starts_with("--"))
            .ok_or("missing option value")?;
        if values.insert(flag.as_str(), value.as_str()).is_some() {
            return Err(format!("duplicate {flag}"));
        }
    }
    let required = |flag| {
        values
            .get(flag)
            .copied()
            .ok_or_else(|| format!("{flag} required"))
    };
    let target: IndividualAddress = required("--target")?
        .parse()
        .map_err(|_| "invalid target address")?;
    let output = Path::new(required("--output")?);
    if output.exists() {
        return Err("output already exists; no overwrite".into());
    }
    let profile = fs::read(required("--profile")?).map_err(|e| e.to_string())?;
    let products = knx_productdb::open_read_only(Path::new(required("--product-db")?))
        .map_err(|e| e.to_string())?;
    let candidate =
        prepare_simulated_candidate(&products.conn, &profile, target).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(&candidate.bundle).map_err(|e| e.to_string())?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(output).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    if fs::read(output).map_err(|e| e.to_string())? != bytes {
        return Err("prepared output readback differs".into());
    }
    println!("STRONG HYPOTHESIS — simulator validation candidate; no bus contact.\nConfiguration: complete product defaults, NO group links or project overrides.\nAll {} source positions; {} allocated octets, {} written, {} NOT written (masked or unused table capacity).\nPlan {}", candidate.source_positions,candidate.allocation_octets,candidate.written_octets,candidate.preserved_octets,candidate.bundle.binding.plan_sha256);
    for inference in &candidate.inferences {
        println!("{inference}");
    }
    println!("Prepared bundle: {}\nPreview it with: knx device system2-validation --bundle {}\nNo real download is enabled. Do not relabel the simulated identity as device evidence.",output.display(),output.display());
    Ok(())
}
