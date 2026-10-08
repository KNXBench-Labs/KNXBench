//! Headless contribution evidence: no bus, user database or external upload.
use knx_app::{contribution, contribution_bundle};
use std::{
    io::{Read, Write},
    path::Path,
    process::ExitCode,
};

pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("contribution refused: {error}");
            ExitCode::FAILURE
        }
    }
}
fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > max {
        return Err("not a regular file or input size limit exceeded".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max {
        return Err("input size limit exceeded".into());
    }
    Ok(bytes)
}
fn execute(args: &[String]) -> Result<(), String> {
    let usage = "knx contribution analyze <file.knxproj|file.knxprod> | preview <file> --options <options.json> | export <file> <out.zip> --options <options.json>";
    let (command, input) = match args {
        [command, input, ..] => (command.as_str(), Path::new(input)),
        _ => return Err(usage.into()),
    };
    let options_path = match (command, args.len()) {
        ("analyze", 2) => None,
        ("preview", 4) if args[2] == "--options" => Some(&args[3]),
        ("export", 5) if args[3] == "--options" => Some(&args[4]),
        _ => return Err(usage.into()),
    };
    let filename = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("input filename is not UTF-8")?;
    if !filename.to_ascii_lowercase().ends_with(".knxproj")
        && !filename.to_ascii_lowercase().ends_with(".knxprod")
    {
        return Err("select .knxproj or .knxprod".into());
    }
    let bytes = read_bounded(input, contribution::MAX_INPUT_BYTES as u64)?;
    if command == "analyze" {
        let analysis = contribution::analyze(&bytes, filename).map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&analysis).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let options: contribution_bundle::BundleOptions =
        serde_json::from_slice(&read_bounded(Path::new(options_path.unwrap()), 8192)?)
            .map_err(|e| e.to_string())?;
    if command == "preview" {
        let preview = contribution_bundle::preview_bundle(&bytes, filename, &options)
            .map_err(|e| e.to_string())?;
        println!(
            "{}",
            serde_json::to_string_pretty(&preview).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let bundle =
        contribution_bundle::build_bundle(&bytes, filename, &options).map_err(|e| e.to_string())?;
    let output = Path::new(&args[2]);
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(&bundle.bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist_noclobber(output).map_err(|e| e.to_string())?;
    println!("evidence ZIP written; not submitted or delivered");
    Ok(())
}
