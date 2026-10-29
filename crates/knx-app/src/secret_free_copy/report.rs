//! Sanitizes primary and selective import reports with one shared semantic policy.

use super::{is_secret_name, Findings, SecretFreeCopy, SecretFreeCopyError, FREE_ACCESS_KEY};
use serde_json::Value;

pub(super) fn copy(report: &Value) -> Result<SecretFreeCopy, SecretFreeCopyError> {
    validate(report).map_err(|resource| SecretFreeCopyError::BudgetExceeded { resource })?;
    let bytes =
        serde_json::to_vec(report).map_err(|_| SecretFreeCopyError::Write("report JSON".into()))?;
    let mut findings = Findings::default();
    let sanitized = match redact_evidence(&bytes, "import report", &mut findings, None) {
        Ok(sanitized) => sanitized.unwrap_or(bytes),
        Err(reason) => {
            findings.ensure()?;
            return Err(SecretFreeCopyError::SourceUnavailable { reason });
        }
    };
    findings.ensure()?;
    findings.finish(sanitized)
}

/// The import evidence envelope embeds the import report as a JSON string.
/// Rows of either that are named like a secret lose their digest, sample
/// and value: a SHA-256 of a four-octet key or a password is reversible by
/// search.
pub(super) fn redact_evidence(
    bytes: &[u8],
    location: &str,
    findings: &mut Findings,
    residual: Option<&super::Needles<'_>>,
) -> Result<Option<Vec<u8>>, &'static str> {
    if bytes.len() > crate::import_evidence::MAX_EVIDENCE_BYTES {
        return Err("import report byte budget exceeded");
    }
    let mut envelope: Value =
        serde_json::from_slice(bytes).map_err(|_| "import evidence is not JSON")?;
    validate(&envelope)?;
    let mut changed = redact_json(&mut envelope, location, findings);
    if let Some(Value::String(embedded)) = envelope.get_mut("report_json") {
        let mut report: Value =
            serde_json::from_str(embedded).map_err(|_| "embedded import report is not JSON")?;
        validate(&report)?;
        let semantic = redact_json(&mut report, location, findings);
        let mut metadata = scrub_metadata(&mut report, &findings.needles())?;
        if let Some(needles) = residual {
            metadata |= scrub_metadata(&mut report, needles)?;
        }
        if metadata {
            findings.redact(location.into(), "import report residual metadata".into());
        }
        if semantic || metadata {
            *embedded = serde_json::to_string(&report)
                .map_err(|_| "embedded import report could not be rewritten")?;
            changed = true;
        }
    }
    let mut metadata = scrub_metadata(&mut envelope, &findings.needles())?;
    if let Some(needles) = residual {
        metadata |= scrub_metadata(&mut envelope, needles)?;
    }
    if metadata {
        findings.redact(location.into(), "import report residual metadata".into());
    }
    changed |= metadata;
    findings.budget.check()?;
    if !changed {
        return Ok(None);
    }
    serde_json::to_vec(&envelope)
        .map(Some)
        .map_err(|_| "import evidence could not be rewritten")
}

fn validate(value: &Value) -> Result<(), &'static str> {
    if !value.is_object() {
        return Err("import report must be an object");
    }
    fn visit(value: &Value, depth: usize, remaining: &mut usize) -> Result<(), &'static str> {
        if depth > 64 {
            return Err("import report depth budget exceeded");
        }
        let size = match value {
            Value::String(text) => text.len().saturating_mul(6),
            _ => 32,
        };
        *remaining = remaining
            .checked_sub(size)
            .ok_or("import report byte budget exceeded")?;
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    *remaining = remaining
                        .checked_sub(key.len().saturating_mul(6))
                        .ok_or("import report byte budget exceeded")?;
                    visit(child, depth + 1, remaining)?;
                }
            }
            Value::Array(items) => {
                for child in items {
                    visit(child, depth + 1, remaining)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let mut remaining = crate::import_evidence::MAX_EVIDENCE_BYTES;
    visit(value, 0, &mut remaining)
}

fn scrub_metadata(value: &mut Value, needles: &super::Needles<'_>) -> Result<bool, &'static str> {
    let mut changed = false;
    match value {
        Value::String(text) if needles.any_in(text.as_bytes()) => {
            *text = String::new();
            changed = true;
        }
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(key.as_str(), "source_path" | "xpath" | "kind" | "name")
                    && child
                        .as_str()
                        .is_some_and(|text| needles.any_in(text.as_bytes()))
                {
                    return Err("secret in historical report row identity; report unavailable");
                }
                if needles.any_in(key.as_bytes()) {
                    return Err("secret in report member identity");
                }
                changed |= scrub_metadata(child, needles)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                changed |= scrub_metadata(child, needles)?;
            }
        }
        _ => {}
    }
    needles.budget.check()?;
    Ok(changed)
}

fn redact_json(value: &mut Value, location: &str, findings: &mut Findings) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = false;
            let connection = map
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| super::local_name(name.as_bytes()) == b"Parameter")
                && map
                    .get("xpath")
                    .and_then(Value::as_str)
                    .is_some_and(|path| {
                        path.rsplit('/').next().is_some_and(|part| {
                            super::local_name(part.split('[').next().unwrap_or(part).as_bytes())
                                == b"BusAccess"
                        })
                    });
            if connection {
                for field in ["sample", "value", "raw_text_preview"] {
                    if let Some(slot) = map.get_mut(field) {
                        if let Some(text) = slot.as_str() {
                            if let Some(clean) =
                                super::xml::redact_connection_string(text, location, findings)
                            {
                                *slot = Value::String(clean);
                                changed = true;
                            }
                        }
                    }
                }
                // Digest-only rows cannot prove their connection string was credential-free.
                if let Some(slot) = map.get_mut("sha256") {
                    if slot.as_str().is_some_and(|s| !s.is_empty()) {
                        *slot = Value::String(String::new());
                        findings.redact(location.into(), "BusAccess/@Parameter digest".into());
                        changed = true;
                    }
                }
            }
            let secret = map
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| is_secret_name(name.as_bytes()))
                .map(str::to_owned);
            if let Some(name) = secret {
                let free_access_digest = knx_store::project_history::image_hash(FREE_ACCESS_KEY);
                let text = |key: &str| map.get(key).and_then(Value::as_str).unwrap_or_default();
                let place = format!(
                    "{location}:{}{}/@{name}",
                    text("source_path"),
                    text("xpath")
                );
                for field in ["sha256", "sample", "value", "raw_text_preview"] {
                    let Some(slot) = map.get_mut(field) else {
                        continue;
                    };
                    if slot.is_null() || slot.as_str() == Some("") {
                        continue;
                    }
                    // The digest of the `BCUKey` free-access sentinel names
                    // no key; it stays, as the value itself does.
                    if field == "sha256"
                        && name == "BCUKey"
                        && slot.as_str() == Some(free_access_digest.as_str())
                    {
                        continue;
                    }
                    if field != "sha256" {
                        if let Some(plain) = slot.as_str() {
                            findings.remember(plain.as_bytes());
                        }
                    }
                    *slot = if field == "raw_text_preview" {
                        Value::Null
                    } else {
                        Value::String(String::new())
                    };
                    findings.redact(place.clone(), format!("{name} {field}"));
                    changed = true;
                }
            }
            for child in map.values_mut() {
                changed |= redact_json(child, location, findings);
            }
            changed
        }
        Value::Array(items) => items.iter_mut().fold(false, |changed, item| {
            redact_json(item, location, findings) | changed
        }),
        _ => false,
    }
}
