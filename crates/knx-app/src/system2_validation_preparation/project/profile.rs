//! Explicit project-only profile grammar; never parsed through v1 defaults.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProjectProductSelector {
    #[serde(rename = "standalone-package-v1")]
    StandalonePackage { package_sha256: String },
    #[serde(rename = "retained-source-members-v1")]
    RetainedSourceMembers { closure_sha256: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectValidationProfile {
    pub format: u16,
    pub id: String,
    pub version: u32,
    pub classification: String,
    pub configuration: String,
    pub program_id: String,
    pub schema_namespace: String,
    pub source_sha256: String,
    pub product_input: ProjectProductSelector,
    pub mask: u16,
    pub manufacturer: u16,
    pub hardware: [u8; 6],
    pub application: [u8; 5],
    pub numeric_order: String,
    pub bit_union_offsets: String,
    pub legacy_patch_always: String,
    pub invisible_parameters: String,
    pub binary_restrictions: String,
    pub reference_activation: String,
    pub procedure_policy: String,
}
pub(super) fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
impl ProjectValidationProfile {
    pub fn from_json(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > 16 * 1024 {
            return Err(invalid("profile exceeds bounded size"));
        }
        let p: Self = serde_json::from_slice(bytes).map_err(io::Error::other)?;
        let selector_hash = match &p.product_input {
            ProjectProductSelector::StandalonePackage { package_sha256 } => package_sha256,
            ProjectProductSelector::RetainedSourceMembers { closure_sha256 } => closure_sha256,
        };
        if p.format != 2
            || p.version != 2
            || p.configuration != CONFIGURATION
            || p.procedure_policy != PROCEDURE
            || !matches!(
                p.schema_namespace.as_str(),
                "http://knx.org/xml/project/20" | "http://knx.org/xml/project/23"
            )
            || p.id.is_empty()
            || p.id.len() > 128
            || p.program_id.is_empty()
            || p.program_id.len() > 1024
            || p.classification != "strong-hypothesis-simulator-validation-only"
            || p.mask != 0x0025
            || !hash(&p.source_sha256)
            || !hash(selector_hash)
            || p.application[..2] != p.manufacturer.to_be_bytes()
            || p.numeric_order != "high-octet-first-explicit-or-schema-default-v1"
            || p.bit_union_offsets != "msb0-byte-aligned-union-relative-v1"
            || p.legacy_patch_always != "inactive-default-ref-consensus-active-wins-v1"
            || p.invisible_parameters != "source-template-inactive-legacy-patch-active-overlay-v1"
            || p.binary_restrictions != "single-octet-enumeration-binary-payload-v1"
            || p.reference_activation != "schema10-no-dynamic-reference-is-active-v1"
        {
            return Err(invalid(
                "unrecognized project candidate profile revision or rules",
            ));
        }
        Ok(p)
    }
    pub(super) fn policy(&self) -> ExperimentalSystem2ImagePolicy {
        ExperimentalSystem2ImagePolicy {
            revision: 2,
            binding: ExperimentalSystem2SourceBinding {
                program_id: self.program_id.clone(),
                schema_namespace: self.schema_namespace.clone(),
                source_sha256: self.source_sha256.clone(),
                package_sha256: match &self.product_input {
                    ProjectProductSelector::StandalonePackage { package_sha256 } => {
                        Some(package_sha256.clone())
                    }
                    ProjectProductSelector::RetainedSourceMembers { .. } => None,
                },
            },
            numeric_order: self.numeric_order.clone(),
            bit_union_offsets: self.bit_union_offsets.clone(),
            legacy_patch_always: self.legacy_patch_always.clone(),
            invisible_parameters: self.invisible_parameters.clone(),
            binary_restrictions: self.binary_restrictions.clone(),
            reference_activation: self.reference_activation.clone(),
        }
    }
    pub(super) fn check_identity(
        &self,
        plan: &knx_core::commissioning::property_download::PropertyDownloadPlan,
    ) -> io::Result<()> {
        use knx_core::commissioning::property_download::PropertyOperation;
        if plan.manufacturer() != self.manufacturer || plan.mask().0 != self.mask
            || !plan.steps().iter().any(|s| matches!(s.operation, PropertyOperation::HardwareCheck { expected } if expected == self.hardware)) {
            return Err(invalid("candidate profile identity differs from translated source"));
        }
        let task = plan.steps().iter().find_map(|s| match &s.operation {
            PropertyOperation::LoadControl { object: 3, payload }
                if payload.octets()[..2] == [3, 2] =>
            {
                Some(&payload.octets()[5..])
            }
            _ => None,
        });
        if task != Some(self.application.as_slice()) {
            return Err(invalid(
                "candidate application identity differs from translated source",
            ));
        }
        Ok(())
    }
}
