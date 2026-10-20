//! Pre-allocation admission without project cloning, source parsing or catalogue writes.
use knx_core::*;

#[cfg(test)]
mod tests;

pub(super) const MAX_ENTITIES: usize = 100_000;
pub(super) const MAX_REFERENCES: usize = 1_000_000;
pub(super) const MAX_INPUT: usize = 32 * 1024 * 1024;
pub(super) const MAX_OUTPUT: usize = 64 * 1024 * 1024;
pub(super) const MAX_DEPTH: usize = 64;

#[derive(Default)]
struct Budget {
    entities: usize,
    bytes: usize,
    references: usize,
}
impl Budget {
    fn entities(&mut self, n: usize) -> Result<(), String> {
        self.entities = self
            .entities
            .checked_add(n)
            .ok_or("entity limit exceeded")?;
        if self.entities > MAX_ENTITIES {
            return Err("entity limit exceeded (100000)".into());
        }
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<(), String> {
        self.bytes = self.bytes.checked_add(n).ok_or("input limit exceeded")?;
        if self.bytes > MAX_INPUT {
            return Err("input limit exceeded (32 MiB)".into());
        }
        Ok(())
    }
    fn refs(&mut self, n: usize) -> Result<(), String> {
        self.references = self
            .references
            .checked_add(n)
            .ok_or("reference limit exceeded")?;
        if self.references > MAX_REFERENCES {
            return Err("reference limit exceeded (1000000)".into());
        }
        self.bytes(n.checked_mul(8).ok_or("input limit exceeded")?)
    }
    fn texts(&mut self, values: &[&str]) -> Result<(), String> {
        for s in values {
            self.bytes(s.len())?;
        }
        Ok(())
    }
    fn source(&mut self, s: &SourceRef) -> Result<(), String> {
        self.texts(&[&s.path, &s.ets_id])
    }
    fn text(&mut self, t: &Override<Text>) -> Result<(), String> {
        match t {
            Override::Value(r) => match &r.value {
                Text::Literal(s) => self.bytes(s.len()),
                Text::Localized(s) => self.bytes(s.0 .0.len()),
            },
            Override::Malformed(s) => self.bytes(s.len()),
            _ => Ok(()),
        }
    }
    fn malformed<T>(&mut self, v: &Override<T>) -> Result<(), String> {
        if let Override::Malformed(s) = v {
            self.bytes(s.len())?;
        }
        Ok(())
    }
}

pub(super) fn admit(project: &Project, options: &super::ExportOptions) -> Result<(), String> {
    let mut b = Budget::default();
    b.texts(&[
        &project.info.project_id,
        &project.info.name,
        project.info.project_number.as_deref().unwrap_or(""),
        &project.strings.default_language().0,
        options.language.as_deref().unwrap_or(""),
    ])?;
    b.entities(project.installations.len())?;
    // Counts are checked before any map/clone. References are charged as eight
    // input bytes apiece so a small entity set cannot carry unbounded edges.
    for i in &project.installations {
        b.entities(i.topology.areas.len())?;
        b.entities(i.topology.lines.len())?;
        b.entities(i.buildings.len())?;
        b.entities(i.group_ranges.len())?;
        b.entities(i.group_addresses.len())?;
        b.entities(i.parameters.len())?;
        b.bytes(i.name.len())?;
        b.refs(i.topology.unassigned.len())?;
        for a in &i.topology.areas {
            b.source(&a.source)?;
            b.bytes(a.name.len())?;
            b.refs(a.lines.len())?;
        }
        for l in &i.topology.lines {
            b.source(&l.source)?;
            b.texts(&[
                &l.name,
                &l.medium_ref,
                l.domain_address.as_deref().unwrap_or(""),
            ])?;
            b.refs(l.devices.len())?;
        }
        for s in &i.buildings {
            b.source(&s.source)?;
            b.texts(&[&s.name, s.number.as_deref().unwrap_or("")])?;
            b.refs(s.children.len())?;
            b.refs(s.devices.len())?;
        }
        for r in &i.group_ranges {
            b.source(&r.source)?;
            b.bytes(r.name.len())?;
            b.refs(r.children.len())?;
        }
        for g in &i.group_addresses {
            b.source(&g.source)?;
            b.bytes(g.name.len())?;
            b.malformed(&g.declared_dpt)?;
        }
        for p in &i.parameters {
            b.source(&p.source)?;
            b.bytes(p.raw.len())?;
        }
    }
    for (k, l, v) in project.strings.iter() {
        b.entities(1)?;
        b.texts(&[&k.0, &l.0, v])?;
    }
    for d in project.devices.iter() {
        b.entities(1)?;
        b.source(&d.source)?;
        b.texts(&[
            &d.name,
            d.description.as_deref().unwrap_or(""),
            &d.product_ref,
            &d.program_ref,
        ])?;
        b.refs(d.com_objects.len())?;
        for binary in &d.binary_data {
            b.entities(1)?;
            b.texts(&[&binary.id, &binary.name])?;
        }
    }
    for o in project.devices.com_objects() {
        b.entities(1)?;
        b.source(&o.source)?;
        b.text(&o.text)?;
        b.text(&o.description)?;
        b.malformed(&o.dpt)?;
        for flag in [
            &o.flags.read,
            &o.flags.write,
            &o.flags.transmit,
            &o.flags.update,
            &o.flags.communication,
            &o.flags.read_on_init,
        ] {
            b.malformed(flag)?;
        }
        b.refs(o.links.len())?;
        if let Some(defaults) = project.devices.program_defaults(o.id) {
            for text in [&defaults.text, &defaults.description]
                .into_iter()
                .flatten()
            {
                match &text.value {
                    Text::Literal(s) => b.bytes(s.len())?,
                    Text::Localized(s) => b.bytes(s.0 .0.len())?,
                }
            }
        }
    }
    for m in project.devices.module_instances() {
        b.entities(1)?;
        b.source(&m.source)?;
        b.texts(&[&m.repeat_index, &m.instance_ets_id])?;
        b.entities(m.arguments.len())?;
        for (s, v) in &m.arguments {
            b.source(s)?;
            b.bytes(v.len())?;
        }
    }
    b.entities(project.functions.len())?;
    for f in &project.functions {
        b.source(&f.source)?;
        b.texts(&[
            &f.details.name,
            f.details.function_type.as_deref().unwrap_or(""),
            f.details.number.as_deref().unwrap_or(""),
            f.details.comment.as_deref().unwrap_or(""),
            f.details.description.as_deref().unwrap_or(""),
        ])?;
        b.entities(f.members.len())?;
        b.refs(f.members.len())?;
        for m in &f.members {
            b.source(&m.source)?;
            b.texts(&[&m.name, m.role.as_deref().unwrap_or("")])?;
        }
    }
    // Excluded heap-bearing fields still count for adapter snapshot cloning.
    // Only lengths are inspected: their contents never enter the report.
    b.entities(project.notes.len())?;
    for note in &project.notes {
        b.texts(&[note.title(), note.body()])?;
    }
    b.entities(project.instructions.len())?;
    for document in project.instructions.values() {
        b.entities(document.runs().len())?;
        b.refs(document.runs().len())?;
        for run in document.runs() {
            b.bytes(run.text.len())?;
        }
    }
    Ok(())
}
