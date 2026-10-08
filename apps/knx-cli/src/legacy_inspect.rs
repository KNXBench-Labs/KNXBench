//! `knx products inspect-legacy`: summarises a legacy EX-IM file and writes nothing.

use std::fmt::Write as _;
use std::io::Read as _;
use std::path::Path;
use std::process::ExitCode;

use knx_app::legacy::{inspect_legacy_file, LegacyPassword};
use knx_productdb::legacy::{
    ExImContent, ExImDiagnostic, LegacyError, LegacyInspection, MAX_LEGACY_FILE,
    PAYLOAD_CHARSET_ASSUMPTION,
};

/// Bytes read from a `--password-file`; far beyond any real password line.
const MAX_PASSWORD_FILE_READ: u64 = 4096;

enum PasswordSource {
    None,
    Stdin,
    File(String),
}

struct Args {
    file: String,
    password: PasswordSource,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut file = None;
    let mut password = PasswordSource::None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--password-stdin" => {
                if !matches!(password, PasswordSource::None) {
                    return Err("give only one of --password-stdin and --password-file".into());
                }
                password = PasswordSource::Stdin;
            }
            "--password-file" => {
                if !matches!(password, PasswordSource::None) {
                    return Err("give only one of --password-stdin and --password-file".into());
                }
                i += 1;
                password = PasswordSource::File(super::take_value(args, i, "--password-file")?);
            }
            // Refused before anything is read, and never repeated back: the
            // value may already be the password.
            flag if flag == "--password" || flag.starts_with("--password=") => {
                return Err(
                    "a password is never accepted on the command line; pass it with \
                     --password-stdin or --password-file <path>"
                        .into(),
                );
            }
            other if other.starts_with("--") => return Err(format!("unknown flag: {other}")),
            other if file.is_none() => file = Some(other.to_string()),
            other => return Err(format!("unexpected extra argument: {other}")),
        }
        i += 1;
    }
    Ok(Args {
        file: file.ok_or("missing <file.vd3|file.vd4|file.vd5|file.pr5>")?,
        password,
    })
}

/// The first line of `text`, one trailing `\r\n` or `\n` removed and
/// nothing else trimmed (a password may start or end with a space).
fn first_line(text: &str) -> Result<LegacyPassword, String> {
    let line = text.split('\n').next().unwrap_or_default();
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.is_empty() {
        return Err("refusing an empty password".into());
    }
    Ok(LegacyPassword::new(line))
}

fn read_password(source: &PasswordSource) -> Result<Option<LegacyPassword>, String> {
    match source {
        PasswordSource::None => Ok(None),
        PasswordSource::Stdin => {
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| format!("failed to read the password from stdin: {e}"))?;
            first_line(&line).map(Some)
        }
        PasswordSource::File(path) => {
            // Only the first line matters; a bounded read keeps a path such
            // as /dev/zero from hanging or exhausting memory.
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .and_then(|file| file.take(MAX_PASSWORD_FILE_READ).read_to_end(&mut bytes))
                .map_err(|e| format!("failed to read the password file {path}: {e}"))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| format!("the password file {path} is not UTF-8 text"))?;
            first_line(&text).map(Some)
        }
    }
}

fn read_bounded(file: &str) -> Result<Vec<u8>, String> {
    let len = std::fs::metadata(file)
        .map_err(|e| format!("failed to read {file}: {e}"))?
        .len();
    if len > MAX_LEGACY_FILE as u64 {
        return Err(format!(
            "{file} is {len} bytes; a legacy EX-IM file may have at most {MAX_LEGACY_FILE}"
        ));
    }
    std::fs::read(file).map_err(|e| format!("failed to read {file}: {e}"))
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    let args = match parse(args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{}", super::USAGE);
            return ExitCode::FAILURE;
        }
    };
    let bytes = match read_bounded(&args.file) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let password = match read_password(&args.password) {
        Ok(password) => password,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match inspect_legacy_file(&bytes, password.as_ref()) {
        Ok(inspection) => {
            print!("{}", format_inspection(&args.file, &inspection));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("inspect-legacy refused {}: {error}", args.file);
            if error == LegacyError::PasswordRequired {
                eprintln!(
                    "rerun with --password-stdin (and type the password) or --password-file <path>"
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn content_label(content: &ExImContent) -> String {
    match content {
        ExImContent::ProductDatabase => "legacy ETS3 product database".into(),
        ExImContent::ProjectExport => "legacy ETS3 project export".into(),
        ExImContent::Other(kind) => format!("legacy ETS3 EX-IM file (content `{kind}`)"),
        ExImContent::Unspecified => "legacy ETS3 EX-IM file (no content line)".into(),
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn diagnostic_line(diagnostic: &ExImDiagnostic) -> String {
    match diagnostic {
        ExImDiagnostic::UnknownHeaderKey { line, key } => {
            format!("unknown header key `{key}` on line {line}")
        }
        ExImDiagnostic::UnknownTypeCode {
            table,
            column,
            type_code,
        } => format!("unknown type code {type_code} for {table}.{column}"),
        ExImDiagnostic::EmptyRequiredValues {
            table,
            column,
            count,
        } => format!("{count} empty value(s) in non-nullable column {table}.{column}"),
        ExImDiagnostic::Windows1252OnlyBytes { count } => format!(
            "{count} byte(s) in 0x80-0x9F, decoded as Windows-1252 (ISO-8859-1 would differ)"
        ),
    }
}

fn format_inspection(file: &str, inspection: &LegacyInspection) -> String {
    let mut out = String::new();
    let name = Path::new(file)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.to_string());
    let _ = writeln!(out, "{}: {name}", content_label(&inspection.content));
    let _ = writeln!(
        out,
        "  container: member {} ({}), {}, {} bytes, sha256 {}",
        inspection.member_name,
        inspection.member_kind.label(),
        if inspection.encrypted {
            "ZipCrypto-encrypted"
        } else {
            "not encrypted"
        },
        inspection.source_len,
        inspection.source_sha256
    );
    let unknown = || "unknown".to_string();
    let _ = writeln!(
        out,
        "  payload: EX-IM format version {}, producer {}, exported {}, {} bytes, sha256 {}",
        inspection.format_version.clone().unwrap_or_else(unknown),
        inspection.producer.clone().unwrap_or_else(unknown),
        inspection.exported_at.clone().unwrap_or_else(unknown),
        inspection.payload_len,
        inspection.payload_sha256
    );
    let _ = writeln!(out, "  text: {PAYLOAD_CHARSET_ASSUMPTION}");
    let _ = writeln!(
        out,
        "  {}, {} ({} joined)",
        plural(inspection.tables.len(), "table", "tables"),
        plural(inspection.total_rows(), "row", "rows"),
        plural(
            inspection.continuation_lines,
            "continuation line",
            "continuation lines"
        )
    );
    for table in &inspection.tables {
        let _ = writeln!(
            out,
            "    {} (T {}): {}, {}",
            table.name,
            table.id,
            plural(table.columns, "column", "columns"),
            plural(table.rows, "row", "rows")
        );
    }
    let _ = writeln!(
        out,
        "  {}:",
        plural(inspection.products.len(), "product", "products")
    );
    for product in &inspection.products {
        let field = |value: &Option<String>| value.clone().unwrap_or_else(|| "?".into());
        let _ = writeln!(
            out,
            "    {}  {} ({}), program {} version {}, mask {}",
            field(&product.order_number),
            field(&product.name),
            field(&product.manufacturer),
            field(&product.program_name),
            field(&product.program_version),
            field(&product.mask_version)
        );
    }
    if inspection.diagnostics.is_empty() {
        let _ = writeln!(out, "  diagnostics: none");
    } else {
        let _ = writeln!(out, "  diagnostics:");
        for diagnostic in &inspection.diagnostics {
            let _ = writeln!(out, "    {}", diagnostic_line(diagnostic));
        }
    }
    let _ = writeln!(
        out,
        "  not imported: inspection only, nothing was written to a product database"
    );
    out
}
