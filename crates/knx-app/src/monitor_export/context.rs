//! Generated-copy admission and shared loss reporting.
use super::{budget::MAX_OUTPUT, ExportIssue};
use knx_core::*;
use serde_json::{json, Map, Value};

#[derive(Default)]
pub(super) struct Context {
    pub issues: Vec<ExportIssue>,
    generated: usize,
}
impl Context {
    /// Conservative retained-copy charge, before JSON strings/maps are built.
    /// Six bytes per input byte covers JSON escaping, plus structural overhead.
    pub fn record(&mut self, texts: &[&str], overhead: usize) -> Result<(), String> {
        let mut charge = overhead;
        for text in texts {
            charge = charge
                .checked_add(
                    text.len()
                        .checked_mul(6)
                        .ok_or("generated output limit exceeded")?,
                )
                .ok_or("generated output limit exceeded")?;
        }
        self.generated = self
            .generated
            .checked_add(charge)
            .ok_or("generated output limit exceeded")?;
        if self.generated > MAX_OUTPUT {
            return Err("generated output limit exceeded (64 MiB conservative copy budget)".into());
        }
        Ok(())
    }
    pub fn issue(
        &mut self,
        code: &str,
        entity: &str,
        field: &str,
        detail: &str,
    ) -> Result<(), String> {
        self.record(&[code, entity, field, detail], 192)?;
        self.issues.push(ExportIssue {
            code: code.into(),
            entity: entity.into(),
            field: field.into(),
            detail: detail.into(),
        });
        Ok(())
    }
    pub fn gap(&mut self, entity: &str, fields: &str) -> Result<(), String> {
        self.issue("unavailable",entity,fields,"Unavailable in current normalized input; empty strings/nulls are sentinels, not measured values.")
    }
}
pub(super) fn dpt(dpt: DptRef) -> Value {
    json!({"main":dpt.main,"sub":dpt.sub})
}
pub(super) fn ga_key(project: &Project, g: &GroupAddressEntry) -> String {
    g.address.format(project.info.group_address_style)
}
pub(super) fn strings(
    ctx: &mut Context,
    values: impl IntoIterator<Item = String>,
) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for v in values {
        ctx.record(&[&v], 16)?;
        result.push(v);
    }
    Ok(result)
}
pub(super) fn text<'a>(
    ctx: &mut Context,
    p: &'a Project,
    language: &Language,
    entity: &str,
    field: &str,
    text: &'a Override<Text>,
) -> Result<&'a str, String> {
    if let Some(value) = text.value() {
        if let Some(s) = p.strings.text(&value.value, language) {
            return Ok(s);
        }
    }
    // Empty is an authored clear, not unavailable text. It is still explicitly
    // disclosed because this wire format cannot retain the four-state source.
    let detail = match text {
        Override::Empty => "Current value explicitly cleared; program defaults were not restored.",
        Override::Malformed(_) => "Current value is malformed; raw source value excluded.",
        _ => "Current text or requested/default StringTable translation unavailable.",
    };
    ctx.issue("text_state", entity, field, detail)?;
    Ok("")
}
pub(super) fn with(mut value: Value, field: &str, child: Value) -> Value {
    if let Value::Object(fields) = &mut value {
        fields.insert(field.into(), child);
    }
    value
}
pub(super) fn map() -> Map<String, Value> {
    Map::new()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_copy_cap_admits_64_mib_and_refuses_next_charge() {
        let mut ctx = Context::default();
        ctx.record(&[], 67_108_864).unwrap();
        assert!(ctx.record(&[], 1).is_err());
    }
}
