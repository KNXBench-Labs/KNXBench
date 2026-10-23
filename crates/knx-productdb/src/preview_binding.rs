//! The immutable binding a future preview/apply compares at apply time.
//!
//! A binding composes the catalogue stamp, the source identity, the policy
//! identity and the complete preview identity into one exact-equality value
//! the server retains between preview and apply. Construction-only: no
//! production route is wired here. See
//! `docs/contracts/catalogue-jobs/managed-binding.md` §5 and
//! `preview-binding-increment.md`.

use crate::catalogue_stamp::CatalogueStamp;
use crate::controlled::{PolicyIdentity, SourceIdentity};
use crate::preview_identity::PreviewIdentity;

/// Format version of a preview binding. Bumped only when a later version
/// changes the binding's field set or equality semantics.
pub(crate) const BINDING_VERSION: u32 = 3;

/// One immutable composition of the catalogue stamp, the source identity, the
/// policy identity and the complete preview identity.
///
/// The fields are the already-admitted inputs (`CatalogueStamp` from a
/// `capture_stamp` reservation, `SourceIdentity` and `PolicyIdentity` from the
/// `controlled` admission projections, `PreviewIdentity` from a rehearsal
/// install); `bind` never re-hashes or re-validates them. Equality is exact
/// field equality: any committed change to the catalogue (via `generation`,
/// `data_version` or the full witness inside the stamp), any difference in
/// source name/length/digest, any different policy or any different rehearsal
/// preview yields a different binding. This is what makes an apply-time
/// comparison stale — it is not a durable commit history or a partial summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewBinding {
    pub(crate) version: u32,
    pub(crate) catalogue: CatalogueStamp,
    pub(crate) source: SourceIdentity,
    pub(crate) policy: PolicyIdentity,
    pub(crate) preview: PreviewIdentity,
}

impl PreviewBinding {
    /// Composes an immutable binding from already-admitted identity inputs.
    ///
    /// Infallible: the inputs are projections of previously validated values,
    /// so composition performs no second hash and no re-admission. The
    /// effective limits are carried by `policy` (`PolicyIdentity` already
    /// projects `max_member_size` and `max_expanded_size`), so no separate
    /// limits field is duplicated here.
    pub(crate) fn bind(
        catalogue: CatalogueStamp,
        source: SourceIdentity,
        policy: PolicyIdentity,
        preview: PreviewIdentity,
    ) -> Self {
        Self {
            version: BINDING_VERSION,
            catalogue,
            source,
            policy,
            preview,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controlled::{ControlledPolicy, ControlledSource};
    use crate::managed_catalogue::{LegacyMutationKind, LegacySettlementHint, ManagedCatalogue};
    use crate::operation::PublicationGate;

    /// A real, migrated schema-28 product DB, since `capture_stamp` runs the
    /// full witness which admits only the supported schema shape.
    fn migrated_owner() -> (tempfile::TempDir, ManagedCatalogue) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = crate::open_and_migrate(&path).unwrap();
        let owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
        (dir, owner)
    }

    fn stamp(owner: &mut ManagedCatalogue) -> CatalogueStamp {
        owner.capture_stamp(&PublicationGate::new()).unwrap()
    }

    fn source_identity(name: &str, bytes: &[u8]) -> SourceIdentity {
        let source = ControlledSource::admit(name, bytes, ControlledPolicy::Standard).unwrap();
        SourceIdentity::from_source(&source)
    }

    /// A fixed, valid preview identity for the composition-only tests below;
    /// the orchestration slice computes the real one from a rehearsal install.
    fn preview() -> PreviewIdentity {
        PreviewIdentity {
            version: 1,
            sha256: [0xAB; 32],
            package_count: 1,
            other_member_count: 0,
            encoded_bytes: 0,
        }
    }

    #[test]
    fn bind_composes_the_catalogue_source_and_policy_exactly() {
        let (_dir, mut owner) = migrated_owner();
        let catalogue = stamp(&mut owner);
        let source = source_identity("pkg.knxprod", b"bytes");
        let policy = PolicyIdentity::from_policy(ControlledPolicy::Large);
        let preview = preview();

        let binding = PreviewBinding::bind(catalogue, source.clone(), policy, preview);

        assert_eq!(binding.version, BINDING_VERSION);
        assert_eq!(binding.catalogue, catalogue);
        assert_eq!(binding.source, source);
        assert_eq!(binding.policy, policy);
        assert_eq!(binding.preview, preview);
    }

    #[test]
    fn the_binding_version_is_pinned() {
        const {
            assert!(BINDING_VERSION == 3);
        }
    }

    #[test]
    fn identical_inputs_produce_equal_bindings() {
        let (_dir, mut owner) = migrated_owner();
        let catalogue = stamp(&mut owner);
        let source = source_identity("a.knxprod", b"bytes");
        let policy = PolicyIdentity::from_policy(ControlledPolicy::Standard);
        let preview = preview();

        let first = PreviewBinding::bind(catalogue, source.clone(), policy, preview);
        let second = PreviewBinding::bind(catalogue, source, policy, preview);

        assert_eq!(first, second);
    }

    #[test]
    fn a_different_source_name_or_bytes_changes_the_binding() {
        let (_dir, mut owner) = migrated_owner();
        let catalogue = stamp(&mut owner);
        let policy = PolicyIdentity::from_policy(ControlledPolicy::Standard);
        let preview = preview();
        let base = PreviewBinding::bind(
            catalogue,
            source_identity("a.knxprod", b"bytes"),
            policy,
            preview,
        );

        let other_name = PreviewBinding::bind(
            catalogue,
            source_identity("b.knxprod", b"bytes"),
            policy,
            preview,
        );
        assert_ne!(base, other_name);

        let other_bytes = PreviewBinding::bind(
            catalogue,
            source_identity("a.knxprod", b"different"),
            policy,
            preview,
        );
        assert_ne!(base, other_bytes);
    }

    #[test]
    fn a_different_policy_changes_the_binding() {
        let (_dir, mut owner) = migrated_owner();
        let catalogue = stamp(&mut owner);
        let source = source_identity("a.knxprod", b"bytes");
        let preview = preview();

        let standard = PreviewBinding::bind(
            catalogue,
            source.clone(),
            PolicyIdentity::from_policy(ControlledPolicy::Standard),
            preview,
        );
        let large = PreviewBinding::bind(
            catalogue,
            source,
            PolicyIdentity::from_policy(ControlledPolicy::Large),
            preview,
        );

        assert_ne!(standard, large);
    }

    #[test]
    fn a_different_preview_identity_changes_the_binding() {
        let (_dir, mut owner) = migrated_owner();
        let catalogue = stamp(&mut owner);
        let source = source_identity("a.knxprod", b"bytes");
        let policy = PolicyIdentity::from_policy(ControlledPolicy::Standard);
        let preview = preview();
        let mut other = preview;
        other.sha256 = [0xCD; 32];

        let base = PreviewBinding::bind(catalogue, source.clone(), policy, preview);
        let changed = PreviewBinding::bind(catalogue, source, policy, other);

        assert_ne!(base, changed);
    }

    #[test]
    fn a_committed_catalogue_change_stales_the_binding() {
        let (_dir, mut owner) = migrated_owner();
        let preview = preview();
        let before = PreviewBinding::bind(
            stamp(&mut owner),
            source_identity("a.knxprod", b"bytes"),
            PolicyIdentity::from_policy(ControlledPolicy::Standard),
            preview,
        );

        // A managed same-W write consumes a generation and changes the witness,
        // so a fresh stamp yields a different catalogue and a different binding.
        owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |conn| {
                    conn.execute(
                        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
                        [],
                    )?;
                    Ok::<_, rusqlite::Error>(())
                },
                |_| LegacySettlementHint::SuccessfulReturn,
            )
            .unwrap()
            .unwrap();
        let after = PreviewBinding::bind(
            stamp(&mut owner),
            source_identity("a.knxprod", b"bytes"),
            PolicyIdentity::from_policy(ControlledPolicy::Standard),
            preview,
        );

        assert_ne!(before, after);
    }
}
