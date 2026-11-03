//! Private provenance-phase facts preserve refusal text without classifying generic errors.
use std::{fmt, io};

// Only the two enforcing source-pin checks construct this marker. Neither an arbitrary
// InvalidData nor a matching message grants fallback eligibility. No serialized vocabulary.
#[derive(Debug)]
pub(super) struct ChangedBinding(pub(super) &'static str);
impl fmt::Display for ChangedBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ChangedBinding {}
pub(super) fn changed(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, ChangedBinding(message))
}
pub(super) enum StrictSource {
    Current,
    ChangedBinding,
}
pub(super) fn outcome(
    result: io::Result<super::hardware::CurrentProjectHardware>,
) -> io::Result<StrictSource> {
    match result {
        Ok(_) => Ok(StrictSource::Current),
        Err(error)
            if error
                .get_ref()
                .is_some_and(|cause| cause.is::<ChangedBinding>()) =>
        {
            Ok(StrictSource::ChangedBinding)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_typed_binding_phase_can_dispatch_changed_source() {
        let message = "caller-pinned source closure differs from winning-source snapshot";
        assert!(matches!(
            outcome(Err(changed(message))).unwrap(),
            StrictSource::ChangedBinding
        ));
        for kind in [
            io::ErrorKind::InvalidData,
            io::ErrorKind::Other,
            io::ErrorKind::NotFound,
            io::ErrorKind::WouldBlock,
        ] {
            assert!(outcome(Err(io::Error::new(kind, message))).is_err());
        }
    }
}
