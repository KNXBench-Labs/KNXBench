//! Bus-side line scan plans: the address range to probe, exclusions built in.
//!
//! An excluded address is baked into candidate construction, never probed
//! and filtered afterwards, never removed from results after the fact —
//! that is T17 Global Constraint 1.

use std::collections::HashSet;
use std::fmt;

use crate::address::{AddressError, IndividualAddress};

/// Packs already-validated `area`/`line`/`device` parts into an address.
/// Used only inside `range()`'s device loop, where `area`/`line` come from
/// `first.area()`/`first.line()` — already four bits by construction of a
/// valid `IndividualAddress` — never from raw, unvalidated `u8` input. If
/// you're tempted to call this from a public entry point that takes raw
/// `u8` area/line, use `IndividualAddress::new` instead and propagate its
/// `AddressError`.
fn packed(area: u8, line: u8, device: u8) -> IndividualAddress {
    IndividualAddress::from_raw(((area as u16) << 12) | ((line as u16) << 8) | device as u16)
}

/// A line scan's candidate list: individual addresses a probe loop may
/// visit. Only ever produced by [`ScanPlanBuilder`] (directly, or via the
/// [`ScanPlan::line`]/[`ScanPlan::range`] shortcuts), which apply the
/// exclusion set while the candidate vector is built — an excluded address
/// is never generated, never probed and filtered afterwards, never
/// removed from results after the fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanPlan {
    addresses: Vec<IndividualAddress>,
    excluded: HashSet<IndividualAddress>,
    omitted: Vec<IndividualAddress>,
}

impl ScanPlan {
    /// Every device address (1..=255) on `area`.`line`. Device 0 is the
    /// line coupler's own address, not a device to probe — see
    /// [`ScanPlanBuilder::line`] for the citation.
    pub fn line(area: u8, line: u8) -> Result<Self, ScanPlanError> {
        ScanPlanBuilder::new().line(area, line)
    }

    /// An explicit `first..=last` device range on one line. Both endpoints
    /// must share an area and line, and `first`'s device must not exceed
    /// `last`'s.
    pub fn range(first: IndividualAddress, last: IndividualAddress) -> Result<Self, ScanPlanError> {
        ScanPlanBuilder::new().range(first, last)
    }

    /// The candidates a probe loop may visit, in ascending device order.
    /// Never contains an excluded address.
    pub fn addresses(&self) -> &[IndividualAddress] {
        &self.addresses
    }

    /// Addresses the requested span contained that this plan left out,
    /// in ascending order. Never empty when something was dropped: spec
    /// §2.1 forbids silently shortening a plan, so a caller that renders
    /// a plan renders this list with it.
    pub fn omitted(&self) -> &[IndividualAddress] {
        &self.omitted
    }

    /// The configuration-supplied exclusion set this plan was built against.
    pub fn excluded(&self) -> &HashSet<IndividualAddress> {
        &self.excluded
    }

    /// Test-only escape hatch: builds a `ScanPlan` directly from its raw
    /// parts, bypassing the exclude-while-building guarantee every public
    /// constructor above enforces. Exists so a downstream crate (T17
    /// Task 3's `knx_net::scan::scan_line`, whose own required test is
    /// "a plan whose `verify()` fails aborts before any frame is sent")
    /// can prove it really calls `verify()` rather than trusting its
    /// caller — something otherwise unreachable from outside this module,
    /// since `addresses`/`excluded` are private and every public
    /// constructor filters exclusions at build time.
    ///
    /// Gated behind the `test-support` feature, which nothing in this
    /// workspace enables outside `[dev-dependencies]`: this function does
    /// not exist in a normal build, so it is not a production bypass of
    /// Global Constraint 1, only a seam for testing code that must defend
    /// against a corrupted plan.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn unchecked_for_tests(
        addresses: Vec<IndividualAddress>,
        excluded: HashSet<IndividualAddress>,
    ) -> Self {
        Self {
            addresses,
            excluded,
            omitted: Vec::new(),
        }
    }

    /// Walks the candidate list once and confirms none of it is excluded.
    /// Cheap enough to call before every scan; a caller that skips this
    /// call and probes `addresses()` directly is not following the
    /// contract this type exists to enforce.
    pub fn verify(&self) -> Result<(), ScanPlanError> {
        for address in &self.addresses {
            if self.excluded.contains(address) {
                return Err(ScanPlanError::ExcludedAddressInRange(*address));
            }
        }
        Ok(())
    }
}

/// Accumulates an exclusion set, then builds a [`ScanPlan`] from it. The
/// only route to a `ScanPlan`; there is no constructor that skips applying
/// `excluded` to the candidate list.
///
/// The builder starts empty. Configuration owners add exclusions through
/// `exclude`/`exclude_all`; those methods only ever add.
#[derive(Debug, Clone, Default)]
pub struct ScanPlanBuilder {
    excluded: HashSet<IndividualAddress>,
}

impl ScanPlanBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one address to the exclusion set.
    pub fn exclude(mut self, address: IndividualAddress) -> Self {
        self.excluded.insert(address);
        self
    }

    /// Adds every address in `addresses` to the exclusion set.
    pub fn exclude_all(mut self, addresses: impl IntoIterator<Item = IndividualAddress>) -> Self {
        self.excluded.extend(addresses);
        self
    }

    /// Every device address (1..=255) on `area`.`line`, excluded addresses
    /// omitted.
    ///
    /// Device 0 is deliberately not in range: it is the line coupler's own
    /// address, not a device on the line. `[D]` `03_05_01 Resources
    /// v01.10.01 AS.md`, §5.3.2.4 `PID_DEVICE_ADDR (PID = 58)`: "The
    /// Property Device Address in the Device Object of the Coupler Model
    /// 2.0 shall be the Device Address part of the own Individual Address
    /// of the Coupler[.] [...] The default [...] value of the Device
    /// Address shall be 00h." Corroborated by §4.5.10.2 "Usage by the
    /// Management Server (device)": "if Device Address = 0 then Device is
    /// Router[.]"
    ///
    /// `area`/`line` are validated through [`IndividualAddress::new`]:
    /// either one out of range (>15) fails with
    /// [`ScanPlanError::InvalidLineAddress`] rather than wrapping around to
    /// a different, unintended line.
    /// A configured excluded address on this line is omitted and reported in
    /// [`ScanPlan::omitted`], not refused: "scan line 1.1" is a standing
    /// sweep of whatever is legitimately reachable, and refusing it
    /// outright would make line 1.1 unscannable for as long as the alarm
    /// panel lives on it. An explicitly enumerated span is the stricter
    /// case and lives in [`ScanPlanBuilder::range`].
    pub fn line(self, area: u8, line: u8) -> Result<ScanPlan, ScanPlanError> {
        let first =
            IndividualAddress::new(area, line, 1).map_err(ScanPlanError::InvalidLineAddress)?;
        let last =
            IndividualAddress::new(area, line, 255).map_err(ScanPlanError::InvalidLineAddress)?;
        self.span(first, last)
    }

    /// An explicit `first..=last` device range on one line. Configured
    /// exclusions are omitted and exposed through [`ScanPlan::omitted`].
    pub fn range(
        self,
        first: IndividualAddress,
        last: IndividualAddress,
    ) -> Result<ScanPlan, ScanPlanError> {
        self.span(first, last)
    }

    fn span(
        self,
        first: IndividualAddress,
        last: IndividualAddress,
    ) -> Result<ScanPlan, ScanPlanError> {
        if first.area() != last.area()
            || first.line() != last.line()
            || first.device() > last.device()
        {
            return Err(ScanPlanError::RangeNotOnOneLine);
        }
        let mut addresses = Vec::new();
        let mut omitted = Vec::new();
        for device in first.device()..=last.device() {
            let address = packed(first.area(), first.line(), device);
            if self.excluded.contains(&address) {
                omitted.push(address);
            } else {
                addresses.push(address);
            }
        }
        if addresses.is_empty() {
            return Err(ScanPlanError::EmptyRange);
        }
        Ok(ScanPlan {
            addresses,
            excluded: self.excluded,
            omitted,
        })
    }
}

/// Why a [`ScanPlan`] could not be built, or failed its own invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanPlanError {
    /// An excluded address reached the candidate list. This is the
    /// assertion Global Constraint 1 demands, as a value.
    ExcludedAddressInRange(IndividualAddress),
    /// Every candidate address in the requested range was excluded.
    EmptyRange,
    /// `first > last`, or the two are not on the same line.
    RangeNotOnOneLine,
    /// `area` or `line` failed `IndividualAddress` validation (out of the
    /// 4-bit range). Returned instead of silently wrapping the value onto
    /// a different, unintended line.
    InvalidLineAddress(AddressError),
}

impl std::error::Error for ScanPlanError {}

impl fmt::Display for ScanPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanPlanError::ExcludedAddressInRange(address) => {
                write!(
                    f,
                    "excluded address {address} reached the scan candidate list"
                )
            }
            ScanPlanError::EmptyRange => {
                write!(
                    f,
                    "scan range is empty: every candidate address was excluded"
                )
            }
            ScanPlanError::RangeNotOnOneLine => {
                write!(
                    f,
                    "scan range endpoints are not a valid first..=last pair on one line"
                )
            }
            ScanPlanError::InvalidLineAddress(err) => {
                write!(f, "invalid line address: {err}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).unwrap()
    }

    #[test]
    fn line_scan_omits_excluded_addresses_and_contains_every_other() {
        // Both omissions come from the caller's configuration.
        let caller_excluded = [addr(1, 1, 5), addr(1, 1, 200)];
        let plan = ScanPlanBuilder::new()
            .exclude_all(caller_excluded)
            .line(1, 1)
            .unwrap();

        assert_eq!(plan.addresses().len(), 255 - 2);
        for device in 1..=255u8 {
            let candidate = addr(1, 1, device);
            let omitted = caller_excluded.contains(&candidate);
            assert_eq!(
                plan.addresses().contains(&candidate),
                !omitted,
                "device {device} should be a candidate iff it was not excluded"
            );
        }
    }

    #[test]
    fn a_new_scan_builder_has_no_installation_specific_exclusions() {
        let plan = ScanPlanBuilder::new().line(2, 2).unwrap();
        assert!(plan.excluded().is_empty());
        assert!(plan.omitted().is_empty());
        assert_eq!(plan.addresses().len(), 255);
    }

    #[test]
    fn device_zero_is_never_a_candidate() {
        // Not even asking to exclude it: it must already be absent because
        // `line()` never generates it in the first place.
        let plan = ScanPlan::line(1, 1).unwrap();
        assert!(!plan.addresses().contains(&addr(1, 1, 0)));
    }

    #[test]
    fn excluding_the_whole_range_yields_empty_range() {
        let all: Vec<IndividualAddress> = (1..=255u8).map(|d| addr(1, 1, d)).collect();
        let err = ScanPlanBuilder::new()
            .exclude_all(all)
            .line(1, 1)
            .unwrap_err();
        assert_eq!(err, ScanPlanError::EmptyRange);
    }

    #[test]
    fn verify_catches_a_smuggled_excluded_address_without_panicking() {
        // Hand-built, bypassing every public constructor: the point is
        // that `verify()` still refuses to hand back Ok on a plan that
        // should never legitimately exist.
        let smuggled = addr(1, 1, 220);
        let plan = ScanPlan {
            addresses: vec![addr(1, 1, 1), smuggled, addr(1, 1, 2)],
            excluded: HashSet::from([smuggled]),
            omitted: Vec::new(),
        };
        assert_eq!(
            plan.verify(),
            Err(ScanPlanError::ExcludedAddressInRange(smuggled))
        );
    }

    #[test]
    fn verify_is_ok_for_a_properly_built_plan() {
        let plan = ScanPlan::line(1, 1).unwrap();
        assert_eq!(plan.verify(), Ok(()));
    }

    #[test]
    fn a_range_spanning_two_lines_is_rejected() {
        let first = addr(1, 1, 1);
        let last = addr(1, 2, 10);
        assert_eq!(
            ScanPlan::range(first, last),
            Err(ScanPlanError::RangeNotOnOneLine)
        );
    }

    #[test]
    fn a_range_with_first_after_last_is_rejected() {
        let first = addr(1, 1, 50);
        let last = addr(1, 1, 10);
        assert_eq!(
            ScanPlan::range(first, last),
            Err(ScanPlanError::RangeNotOnOneLine)
        );
    }

    #[test]
    fn configured_exclusions_are_omitted_from_an_explicit_range() {
        let protected = addr(2, 3, 42);
        let plan = ScanPlanBuilder::new()
            .exclude(protected)
            .range(addr(2, 3, 40), addr(2, 3, 44))
            .unwrap();

        assert!(!plan.addresses().contains(&protected));
        assert_eq!(plan.omitted(), &[protected]);
        assert_eq!(plan.verify(), Ok(()));
    }

    #[test]
    fn area_out_of_range_is_rejected_rather_than_masked_to_a_different_line() {
        // Before this fix, area 16 masked to area 0 (16 & 0x0F == 0) and
        // silently returned a plan on line 0.1.x instead of failing. It
        // must now fail, and there must be no `Ok` plan to inspect at all.
        assert!(matches!(
            ScanPlan::line(16, 1),
            Err(ScanPlanError::InvalidLineAddress(_))
        ));
    }

    #[test]
    fn line_out_of_range_is_rejected_rather_than_masked_to_a_different_line() {
        // Same failure mode as above, for the line nibble: 16 would have
        // masked to line 0.
        assert!(matches!(
            ScanPlan::line(1, 16),
            Err(ScanPlanError::InvalidLineAddress(_))
        ));
    }
}
