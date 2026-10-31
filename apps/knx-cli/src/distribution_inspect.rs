//! Emits read-only distribution-inspection JSON without opening a product database or output file.

use knx_productdb::distribution::{inspect_distribution, DistributionLimits};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

const USAGE:&str="usage: knx products inspect-distribution <source.xml|source.zip>\nRead-only JSON source inspection; no installation, code execution or write authority.";

pub(crate) fn run(args: &[String]) -> ExitCode {
    // Validate the complete invocation before opening any path. No DB/output/execution flags.
    if args.len() != 1 || args[0].starts_with('-') || args[0].is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    match inspect_file(Path::new(&args[0])) {
        Ok(report) => {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            if let Err(e) = serde_json::to_writer(&mut out, &report)
                .and_then(|()| out.write_all(b"\n").map_err(serde_json::Error::io))
            {
                eprintln!(
                    "{}",
                    serde_json::json!({"error":{"reason":"output","detail":e.to_string()}})
                );
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
fn inspect_file(
    path: &Path,
) -> Result<knx_productdb::distribution::DistributionInspection, serde_json::Value> {
    let io_error = |e: std::io::Error| serde_json::json!({"error":{"reason":"sourceRead","detail":e.to_string()}});
    let initial = std::fs::metadata(path).map_err(io_error)?;
    if !initial.is_file() {
        return Err(serde_json::json!({"error":{"reason":"sourceNotRegular"}}));
    }
    let file = File::open(path).map_err(io_error)?;
    let before = file.metadata().map_err(io_error)?;
    let max = DistributionLimits::default().max_input_bytes as u64;
    if !before.is_file() {
        return Err(serde_json::json!({"error":{"reason":"sourceNotRegular"}}));
    }
    if before.len() > max {
        return Err(serde_json::json!({"error":{"reason":"resourceLimit","detail":"input bytes"}}));
    }
    let mut data = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut data)
        .map_err(io_error)?;
    if data.len() as u64 > max {
        return Err(serde_json::json!({"error":{"reason":"resourceLimit","detail":"input bytes"}}));
    }
    if data.len() as u64 != before.len() {
        return Err(serde_json::json!({"error":{"reason":"sourceChangedDuringRead"}}));
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    inspect_distribution(&name, &data).map_err(|error| serde_json::json!({"error":error}))
}
