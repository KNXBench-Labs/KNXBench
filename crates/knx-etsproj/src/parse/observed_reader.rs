//! Count a fixed lexical vocabulary in the authoritative parser stream.
//!
//! Includes retained subtrees; observations are not semantic admission.
use quick_xml::{events::Event, name::QName, XmlVersion};
use std::collections::BTreeMap;

pub(crate) struct ObservedReader<R> {
    inner: quick_xml::Reader<R>,
    pub counts: BTreeMap<String, u32>,
    // Only declarations at each scope, not cloned inherited maps. This keeps
    // namespace storage linear in the bounded source, even for deep documents.
    namespaces: Vec<BTreeMap<String, String>>,
    pending_pop: bool,
}
impl<'a> ObservedReader<&'a [u8]> {
    pub fn from_reader(input: &'a [u8]) -> Self {
        Self {
            inner: quick_xml::Reader::from_reader(input),
            counts: BTreeMap::new(),
            namespaces: Vec::new(),
            pending_pop: false,
        }
    }
    pub fn is_project_element(&self, name: QName<'_>, version: u32) -> bool {
        let name = name.as_ref();
        let prefix = name.split_once(':').map_or("", |(prefix, _)| prefix);
        if matches!(prefix, "xml" | "xmlns") {
            return false;
        }
        self.namespaces
            .iter()
            .rev()
            .find_map(|scope| scope.get(prefix))
            .is_some_and(|namespace| namespace == &format!("http://knx.org/xml/project/{version}"))
    }
    pub fn buffer_position(&self) -> u64 {
        self.inner.buffer_position()
    }
    pub fn read_event(&mut self) -> Result<Event<'a>, quick_xml::Error> {
        if self.pending_pop {
            self.namespaces.pop();
            self.pending_pop = false;
        }
        let event = self.inner.read_event()?;
        if let Event::Start(e) | Event::Empty(e) = &event {
            let mut scope = BTreeMap::new();
            for attr in e.attributes() {
                let attr = attr?;
                let key = attr.key.into_inner();
                let prefix = if key == "xmlns" {
                    Some("")
                } else {
                    key.strip_prefix("xmlns:")
                };
                if let Some(prefix) = prefix {
                    // NsReader's resolver sees raw entity spellings in xmlns.
                    // Match detection/attribute parsing by decoding XML values first.
                    let value = attr.normalized_value(XmlVersion::Implicit1_0)?;
                    scope.insert(prefix.to_string(), value.into_owned());
                }
            }
            self.namespaces.push(scope);
            self.pending_pop = matches!(event, Event::Empty(_));
            let qname = e.name();
            let name = qname.local_name();
            let name = name.as_ref();
            if matches!(
                name,
                "Installation"
                    | "Area"
                    | "Line"
                    | "DeviceInstance"
                    | "ParameterInstanceRef"
                    | "ModuleInstance"
                    | "Argument"
                    | "ComObjectInstanceRef"
                    | "GroupObjectTree"
                    | "Node"
                    | "GroupAddress"
                    | "GroupRange"
                    | "Space"
                    | "BuildingPart"
                    | "DeviceInstanceRef"
            ) {
                let count = self.counts.entry(name.to_string()).or_default();
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| std::io::Error::other("XML observation counter overflow"))?;
            }
        } else if matches!(event, Event::End(_)) {
            self.pending_pop = true;
        }
        Ok(event)
    }
    pub fn read_to_end(&mut self, end: QName<'_>) -> Result<(), quick_xml::Error> {
        let mut depth = 0usize;
        loop {
            match self.read_event()? {
                Event::Start(_) => depth += 1,
                Event::End(e) if depth == 0 && e.name() == end => return Ok(()),
                Event::End(_) if depth > 0 => depth -= 1,
                Event::Eof => {
                    return Err(quick_xml::Error::IllFormed(
                        quick_xml::errors::IllFormedError::MissingEndTag(end.as_ref().to_string()),
                    ))
                }
                _ => {}
            }
        }
    }
}
