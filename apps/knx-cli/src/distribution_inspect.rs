//! Emits read-only JSON or owned offline HTML without opening a product database or output file.

#[path = "distribution_html.rs"]
mod html;

use knx_productdb::distribution::{inspect_distribution, DistributionLimits};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

const USAGE:&str="usage: knx products inspect-distribution <source.xml|source.zip> [--output-format json|html]\nRead-only offline source inspection; no installation, code execution or write authority.";

fn arguments(args: &[String]) -> Option<(&str, &str)> {
    let (path, format) = match args {
        [path] => (path.as_str(), "json"),
        [path, flag, format] if flag == "--output-format" => (path.as_str(), format.as_str()),
        [flag, format, path] if flag == "--output-format" => (path.as_str(), format.as_str()),
        _ => return None,
    };
    (!path.is_empty() && !path.starts_with('-') && matches!(format, "json" | "html"))
        .then_some((path, format))
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    // Validate the complete invocation before opening any path. No DB/output/execution flags.
    let Some((path, format)) = arguments(args) else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match inspect_file(Path::new(path)) {
        Ok(report) => {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let result = if format == "html" {
                out.write_all(html::render(&report).as_bytes())
                    .map_err(serde_json::Error::io)
            } else {
                serde_json::to_writer(&mut out, &report)
                    .and_then(|()| out.write_all(b"\n").map_err(serde_json::Error::io))
            };
            if let Err(e) = result {
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
