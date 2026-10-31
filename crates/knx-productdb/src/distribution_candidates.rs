//! Reports exact TypeNumber value and Union member-interval source candidates without repair.

use super::dependencies::{named, presence};
use super::{readers::ReferenceIndex, xml::Document, *};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use DistributionCandidateKind as Kind;

/// Lexical ceiling for one numeric attribute; longer text is refused work, not a source error.
const MAX_NUMERIC_LEXICAL_BYTES: usize = 128;
/// Largest admitted decimal exponent magnitude; beyond it the value is refused work.
const MAX_DECIMAL_EXPONENT: i64 = 1_000_000;
/// Overlap pairs listed per Union; the exact pair count is always reported.
const MAX_OVERLAP_SAMPLES: usize = 16;

/// One source-candidate diagnostic: a declaration fact, never a runtime error or correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSourceCandidate {
    pub kind: DistributionCandidateKind,
    pub severity: DistributionCandidateSeverity,
    /// The value owner (Parameter or ParameterRef), the Union member Parameter, or the Union.
    pub source: DistributionSourceRef,
    pub application_source: DistributionSourceRef,
    pub manufacturer_source: DistributionSourceRef,
    /// Nearest enclosing ModuleDef definition; `None` means the application scope itself.
    pub module_definition_source: Option<DistributionSourceRef>,
    /// `ParameterRef/@RefId`, only for `parameterRefValue` rows.
    pub parameter: Option<DistributionCandidateTarget>,
    /// `Parameter/@ParameterType` of the value owner or member; `None` for Union summaries.
    pub parameter_type: Option<DistributionCandidateTarget>,
    pub value_check: Option<DistributionValueCheck>,
    pub interval_check: Option<DistributionIntervalCheck>,
    pub union_summary: Option<DistributionUnionSummary>,
    pub activity: DistributionCandidateActivity,
    pub initialization: DistributionCandidateInitialization,
    pub dependency_context: DistributionCandidateDependencies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionCandidateKind {
    /// `Parameter/@Value` (or its absence) of a TypeNumber-or-unknown-type Parameter.
    ParameterValue,
    /// A present `ParameterRef/@Value` override; refs without `Value` get no row.
    ParameterRefValue,
    UnionMemberInterval,
    UnionIntervalSummary,
}

/// Always `sourceCandidate`: a mathematical observation about declarations, not a defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionCandidateSeverity {
    SourceCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionCandidateInitialization {
    NotPerformed,
}

/// Named arithmetic model; no schema, ETS or device behaviour is implied by either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributionArithmeticModel {
    #[serde(rename = "finite-decimal-comparison-v1")]
    FiniteDecimalComparisonV1,
    #[serde(rename = "union-member-relative-bits-v1")]
    UnionMemberRelativeBitsV1,
}

/// A declared reference resolved in the original scope; candidates keep every duplicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCandidateTarget {
    pub attribute: String,
    pub presence: DistributionPresence,
    pub raw_ref: Option<String>,
    /// `None` only when the attribute is absent; an empty reference is `dangling`.
    pub reference_state: Option<DistributionReferenceState>,
    pub candidates: Vec<DistributionSourceRef>,
    pub basis: Option<DistributionResolutionBasis>,
}

/// Lexical class under `finite-decimal-comparison-v1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionDecimalClass {
    Absent,
    Empty,
    FiniteDecimal,
    /// Not `[+-]digits[.digits][(e|E)[+-]digits]`, e.g. `NaN`, `INF`, `bad` or inner spaces.
    NotFiniteDecimal,
    /// Longer than the lexical cap or exponent beyond the cap: refused work, not a source error.
    ExceedsLexicalLimit,
}

/// An attribute's XML-decoded value (not its byte-exact spelling) and its exact decimal reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDecimalField {
    pub attribute: String,
    pub presence: DistributionPresence,
    pub value: Option<String>,
    pub class: DistributionDecimalClass,
    /// `[-]<digits>e<exponent>` without leading/trailing zeros, or `0`; only for finite decimals.
    pub normalized: Option<String>,
}

/// Outcome of one closed-bound side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionSideComparison {
    Outside,
    NotOutside,
    UnboundedOnThatSide,
    /// The value or this bound is not a usable finite decimal, or the range is reversed.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionValueComparison {
    /// No unique TypeNumber: missing, dangling or ambiguous type or Parameter, or no single Type*.
    TypeUnavailable,
    ValueAbsent,
    ValueEmpty,
    NotFiniteDecimal,
    LexicalResourceLimit,
    /// Both bounds valid and minimum > maximum; no inside/outside assertion is made.
    InvalidDeclaredRange,
    Below,
    Above,
    /// A present bound is not a finite decimal; that side is not compared.
    BoundInvalid,
    /// No bound is declared: there is no bounded-range assurance.
    Unbounded,
    Within,
}

/// Where the TypeNumber came from; other unique type kinds produce no row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionTypeShape {
    TypeNumber,
    ParameterUnavailable,
    TypeReferenceUnavailable,
    NoTypeElement,
    MultipleTypeElements,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionValueCheck {
    pub model: DistributionArithmeticModel,
    pub type_shape: DistributionTypeShape,
    /// The unique TypeNumber element; `None` unless `typeShape` is `typeNumber`.
    pub type_source: Option<DistributionSourceRef>,
    /// `Type` and `SizeInBit` of the TypeNumber, XML-decoded; unconverted, never applied.
    pub type_attributes: Vec<DistributionDeclaredAttribute>,
    /// `Value` on `source`.
    pub value: DistributionDecimalField,
    /// `minInclusive` / `maxInclusive` on `typeSource`.
    pub minimum: DistributionDecimalField,
    pub maximum: DistributionDecimalField,
    pub below_minimum: DistributionSideComparison,
    pub above_maximum: DistributionSideComparison,
    pub comparison: DistributionValueComparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionIntegerClass {
    Absent,
    Empty,
    Integer,
    NotInteger,
    /// Beyond the lexical cap or the signed 64-bit range: refused work, not a source error.
    ExceedsLimit,
}

/// An integer attribute's XML-decoded value and exact reading; the owner follows from context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionIntegerField {
    pub attribute: String,
    pub presence: DistributionPresence,
    pub value: Option<String>,
    pub class: DistributionIntegerClass,
    pub parsed: Option<i64>,
}

/// Half-open member-relative bit interval `[startBit, endBit)`; never an absolute address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionBitInterval {
    pub start_bit: i64,
    pub end_bit: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionIntervalComparison {
    WithinExtent,
    OutsideExtent,
    OffsetAbsent,
    BitOffsetAbsent,
    OffsetInvalid,
    BitOffsetInvalid,
    /// No unique type, or its Type* element declares no `SizeInBit`.
    WidthUnknown,
    /// `SizeInBit` is not an integer or is negative.
    WidthInvalid,
    ExtentUnknown,
    ExtentInvalid,
    ArithmeticLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionIntervalCheck {
    pub model: DistributionArithmeticModel,
    pub union_source: DistributionSourceRef,
    /// Union Memory/Property children: context only, never added to member intervals.
    pub union_placement: Vec<DistributionSourceRef>,
    /// The unique Type* element whose `SizeInBit` is the width.
    pub type_source: Option<DistributionSourceRef>,
    /// Member `Offset` (octets) and `BitOffset` (bits) on `source`; omission is not zero.
    pub offset: DistributionIntegerField,
    pub bit_offset: DistributionIntegerField,
    pub width: DistributionIntegerField,
    /// `Union/@SizeInBit`.
    pub extent: DistributionIntegerField,
    pub interval: Option<DistributionBitInterval>,
    /// Explicit `SizeInBit="0"`: a known empty interval, no encoding assurance.
    pub zero_width_declared: bool,
    pub comparison: DistributionIntervalComparison,
}

/// One distinct known member interval and how many member declarations share it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionDistinctInterval {
    pub start_bit: i64,
    pub end_bit: i64,
    pub members: u64,
}

/// Indices into `distinctIntervals` of two overlapping non-empty intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionIntervalPair {
    pub first: u32,
    pub second: u32,
}

/// Declaration counts and distinct intervals; overlap is a relation, not simultaneous writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionUnionSummary {
    pub model: DistributionArithmeticModel,
    pub extent: DistributionIntegerField,
    pub member_declarations: u64,
    pub known_intervals: u64,
    pub unavailable_intervals: u64,
    pub unknown_width_members: u64,
    pub outside_members: u64,
    /// Sorted by start, then end.
    pub distinct_intervals: Vec<DistributionDistinctInterval>,
    /// Exact count of overlapping pairs of distinct non-empty intervals.
    pub overlapping_distinct_pairs: u64,
    pub overlap_samples: Vec<DistributionIntervalPair>,
    pub overlap_samples_truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionActivityState {
    /// The application's Dynamic subset was not evaluated.
    NotEvaluated,
    ObservedInEvaluatedSubset,
    /// See `reason`; never a known-inactive verdict.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionActivityReason {
    DynamicNotAttempted,
    /// The evaluated subset is incomplete, so absence there says nothing about activity.
    NotObservedInEvaluatedSubset,
    /// No ParameterRef resolves uniquely to this Parameter.
    NoUniqueParameterRef,
    /// Union summaries aggregate declarations; activity belongs to member rows.
    SummaryRow,
}

/// What the evaluated subset was computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DistributionActivityInput {
    SourceFallbacks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionActivityObservation {
    pub parameter_ref_source: DistributionSourceRef,
    pub module_scope: Vec<DistributionModuleScope>,
    pub input: DistributionActivityInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCandidateActivity {
    pub state: DistributionActivityState,
    pub reason: Option<DistributionActivityReason>,
    pub observations: Vec<DistributionActivityObservation>,
}

/// A dependency declaration naming a ParameterRef of this row; no value is calculated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCandidateDependency {
    pub kind: DistributionDependencyKind,
    pub role: DistributionEndpointRole,
    pub source: DistributionSourceRef,
    pub parameter_ref_source: DistributionSourceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionCandidateDependencies {
    /// `applicationImpactUnknown` when the application declares any script, calculation,
    /// validation or handler; never a non-impact guarantee.
    pub impact_scope: DistributionImpactScope,
    pub declarations: Vec<DistributionCandidateDependency>,
}

/// XML Schema whitespace that `xs:decimal`/`xs:integer` collapse around a lexical value.
fn trim_xsd(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\n', '\r'])
}

/// An exact finite decimal `digits * 10^exponent`; `digits` has no leading or trailing zero.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Exact {
    negative: bool,
    digits: Vec<u8>,
    exponent: i64,
}
impl Exact {
    fn normalized(&self) -> String {
        if self.digits.is_empty() {
            return "0".into();
        }
        let digits = String::from_utf8_lossy(&self.digits);
        let sign = if self.negative { "-" } else { "" };
        format!("{sign}{digits}e{}", self.exponent)
    }
    fn signum(&self) -> i8 {
        match (self.digits.is_empty(), self.negative) {
            (true, _) => 0,
            (false, true) => -1,
            (false, false) => 1,
        }
    }
    /// Exact order without binary floating point or integer narrowing.
    fn compare(&self, other: &Self) -> Ordering {
        let (a, b) = (self.signum(), other.signum());
        if a != b || a == 0 {
            return a.cmp(&b);
        }
        let leading = |e: &Self| e.digits.len() as i64 + e.exponent;
        let magnitude = leading(self)
            .cmp(&leading(other))
            .then_with(|| self.digits.cmp(&other.digits));
        if a < 0 {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
}

/// `[+-]digits[.digits][(e|E)[+-]digits]` with at least one mantissa digit, after XSD trimming.
fn parse_decimal(raw: &str) -> std::result::Result<Exact, DistributionDecimalClass> {
    use DistributionDecimalClass as C;
    if raw.is_empty() {
        return Err(C::Empty);
    }
    if raw.len() > MAX_NUMERIC_LEXICAL_BYTES {
        return Err(C::ExceedsLexicalLimit);
    }
    let text = trim_xsd(raw).as_bytes();
    let mut at = 0;
    let negative = match text.first() {
        Some(b'-') => {
            at = 1;
            true
        }
        Some(b'+') => {
            at = 1;
            false
        }
        _ => false,
    };
    let digits_from = |at: &mut usize| {
        let start = *at;
        while text.get(*at).is_some_and(u8::is_ascii_digit) {
            *at += 1;
        }
        &text[start..*at]
    };
    let integer = digits_from(&mut at);
    let fraction = if text.get(at) == Some(&b'.') {
        at += 1;
        digits_from(&mut at)
    } else {
        &[][..]
    };
    if integer.is_empty() && fraction.is_empty() {
        return Err(C::NotFiniteDecimal);
    }
    let mut exponent = 0i64;
    let mut exponent_overflow = false;
    if matches!(text.get(at), Some(b'e' | b'E')) {
        at += 1;
        let sign = match text.get(at) {
            Some(b'-') => {
                at += 1;
                -1
            }
            Some(b'+') => {
                at += 1;
                1
            }
            _ => 1,
        };
        let digits = digits_from(&mut at);
        if digits.is_empty() {
            return Err(C::NotFiniteDecimal);
        }
        for d in digits {
            exponent = exponent * 10 + i64::from(d - b'0');
            if exponent > MAX_DECIMAL_EXPONENT {
                exponent_overflow = true;
                break;
            }
        }
        exponent *= sign;
    }
    if at != text.len() {
        return Err(C::NotFiniteDecimal);
    }
    if exponent_overflow {
        return Err(C::ExceedsLexicalLimit);
    }
    let mut digits = [integer, fraction].concat();
    // Bounded by the lexical ceiling, so these conversions cannot overflow.
    let mut exponent = exponent - fraction.len() as i64;
    let leading = digits.iter().take_while(|&&d| d == b'0').count();
    digits.drain(..leading);
    while digits.last() == Some(&b'0') {
        digits.pop();
        exponent += 1;
    }
    if digits.is_empty() {
        return Ok(Exact {
            negative: false,
            digits,
            exponent: 0,
        });
    }
    Ok(Exact {
        negative,
        digits,
        exponent,
    })
}

fn decimal_field(
    doc: &Document<'_>,
    owner: Option<usize>,
    attribute: &str,
) -> (DistributionDecimalField, Option<Exact>) {
    let value = owner.and_then(|o| doc.nodes[o].attr(attribute));
    let parsed = value.as_deref().map(parse_decimal);
    let class = match &parsed {
        None => DistributionDecimalClass::Absent,
        Some(Ok(_)) => DistributionDecimalClass::FiniteDecimal,
        Some(Err(class)) => *class,
    };
    let exact = parsed.and_then(std::result::Result::ok);
    let field = DistributionDecimalField {
        attribute: attribute.into(),
        presence: presence(&value),
        value,
        class,
        normalized: exact.as_ref().map(Exact::normalized),
    };
    (field, exact)
}

fn parse_integer(raw: &str) -> std::result::Result<i64, DistributionIntegerClass> {
    use DistributionIntegerClass as C;
    if raw.is_empty() {
        return Err(C::Empty);
    }
    if raw.len() > MAX_NUMERIC_LEXICAL_BYTES {
        return Err(C::ExceedsLimit);
    }
    let text = trim_xsd(raw);
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(C::NotInteger);
    }
    let mut value = 0i64;
    for d in digits.bytes() {
        let d = i64::from(d - b'0');
        value = value
            .checked_mul(10)
            .and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            })
            .ok_or(C::ExceedsLimit)?;
    }
    Ok(value)
}

fn integer_field(
    doc: &Document<'_>,
    owner: Option<usize>,
    attribute: &str,
) -> DistributionIntegerField {
    let value = owner.and_then(|o| doc.nodes[o].attr(attribute));
    let parsed = value.as_deref().map(parse_integer);
    DistributionIntegerField {
        attribute: attribute.into(),
        presence: presence(&value),
        class: match &parsed {
            None => DistributionIntegerClass::Absent,
            Some(Ok(_)) => DistributionIntegerClass::Integer,
            Some(Err(class)) => *class,
        },
        parsed: parsed.and_then(std::result::Result::ok),
        value,
    }
}

/// The unique `Type*` child of a ParameterType, or why there is none.
fn type_element(
    doc: &Document<'_>,
    parameter_type: usize,
) -> std::result::Result<usize, DistributionTypeShape> {
    let mut found = doc.children(parameter_type).filter(|&c| {
        let n = &doc.nodes[c];
        n.ns.as_ref() == doc.namespace && n.local.starts_with("Type")
    });
    match (found.next(), found.next()) {
        (None, _) => Err(DistributionTypeShape::NoTypeElement),
        (Some(one), None) => Ok(one),
        (Some(_), Some(_)) => Err(DistributionTypeShape::MultipleTypeElements),
    }
}

/// Declarations named by `owner/@attribute` in the original scope, like `references[]`.
fn lookup(
    doc: &Document<'_>,
    index: &ReferenceIndex,
    owner: usize,
    attribute: &str,
    kind: &str,
) -> (Option<String>, Vec<usize>, DistributionResolutionBasis) {
    let raw = doc.nodes[owner].attr(attribute);
    let Some(value) = raw.clone() else {
        return (None, Vec::new(), DistributionResolutionBasis::OriginalScope);
    };
    let find = |scope| {
        index
            .get(&(
                doc.manufacturer(owner),
                scope,
                kind.to_string(),
                value.clone(),
            ))
            .cloned()
            .unwrap_or_default()
    };
    let scope = doc.entity_scope(owner);
    let found = find(scope);
    // Module parameters may name application-level types, exactly as `references[]` resolves.
    if found.is_empty()
        && kind == "ParameterType"
        && scope.is_some_and(|s| doc.nodes[s].local == "ModuleDef")
    {
        let enclosing = find(doc.application(owner));
        if !enclosing.is_empty() {
            return (
                raw,
                enclosing,
                DistributionResolutionBasis::EnclosingApplicationScope,
            );
        }
    }
    (raw, found, DistributionResolutionBasis::OriginalScope)
}

/// The resolved type of a Parameter: the unique ParameterType and its unique `Type*`, if any.
struct ParameterTyping {
    raw: Option<String>,
    candidates: Vec<usize>,
    basis: DistributionResolutionBasis,
    element: std::result::Result<usize, DistributionTypeShape>,
}
fn typing(doc: &Document<'_>, index: &ReferenceIndex, parameter: usize) -> ParameterTyping {
    let (raw, candidates, basis) = lookup(doc, index, parameter, "ParameterType", "ParameterType");
    let element = match candidates.as_slice() {
        [one] => type_element(doc, *one),
        _ => Err(DistributionTypeShape::TypeReferenceUnavailable),
    };
    ParameterTyping {
        raw,
        candidates,
        basis,
        element,
    }
}
impl ParameterTyping {
    /// Rows are made for TypeNumber and for types whose kind cannot be determined.
    fn eligible(&self, doc: &Document<'_>) -> bool {
        self.element
            .map_or(true, |e| doc.nodes[e].local == "TypeNumber")
    }
}

struct Pass<'a, 'd> {
    doc: &'a Document<'d>,
    references: &'a mut usize,
    max_references: usize,
    /// ParameterRefs whose `RefId` resolves to exactly this Parameter, in element order.
    refs_by_parameter: HashMap<usize, Vec<usize>>,
    /// Applications whose Dynamic subset was evaluated.
    evaluated: HashSet<DistributionSourceRef>,
    /// (application, ref Id, innermost ModuleDef Id) -> activation scope chains, evaluator order.
    active:
        HashMap<(DistributionSourceRef, String, Option<String>), Vec<Vec<DistributionModuleScope>>>,
    dependencies: HashMap<DistributionSourceRef, Vec<DistributionCandidateDependency>>,
    opaque: HashSet<DistributionSourceRef>,
}

impl Pass<'_, '_> {
    /// Each target and candidate is charged to the shared reference budget before copying.
    fn target(
        &mut self,
        attribute: &str,
        (raw, found, basis): (Option<String>, &[usize], DistributionResolutionBasis),
    ) -> Result<DistributionCandidateTarget> {
        *self.references = self
            .references
            .checked_add(1 + found.len())
            .filter(|&n| n <= self.max_references)
            .ok_or_else(|| limit("references"))?;
        Ok(DistributionCandidateTarget {
            attribute: attribute.into(),
            presence: presence(&raw),
            reference_state: raw.as_ref().map(|_| match found.len() {
                0 => DistributionReferenceState::Dangling,
                1 => DistributionReferenceState::Resolved,
                _ => DistributionReferenceState::Ambiguous,
            }),
            basis: raw.as_ref().map(|_| basis),
            raw_ref: raw,
            candidates: found
                .iter()
                .map(|&i| self.doc.coordinate(i))
                .collect::<Result<_>>()?,
        })
    }

    fn type_target(&mut self, typing: &ParameterTyping) -> Result<DistributionCandidateTarget> {
        self.target(
            "ParameterType",
            (typing.raw.clone(), &typing.candidates, typing.basis),
        )
    }

    fn activity(
        &self,
        app: DistributionSourceRef,
        refs: &[usize],
    ) -> DistributionCandidateActivity {
        if !self.evaluated.contains(&app) {
            return DistributionCandidateActivity {
                state: DistributionActivityState::NotEvaluated,
                reason: Some(DistributionActivityReason::DynamicNotAttempted),
                observations: Vec::new(),
            };
        }
        let mut observations = Vec::new();
        for &r in refs {
            let doc = self.doc;
            let Some(id) = doc.nodes[r].attr("Id") else {
                continue;
            };
            let module = doc
                .entity_scope(r)
                .filter(|&s| doc.nodes[s].local == "ModuleDef")
                .and_then(|s| doc.nodes[s].attr("Id"));
            let Ok(source) = doc.coordinate(r) else {
                continue;
            };
            for scope in self.active.get(&(app, id, module)).into_iter().flatten() {
                observations.push(DistributionActivityObservation {
                    parameter_ref_source: source,
                    module_scope: scope.clone(),
                    input: DistributionActivityInput::SourceFallbacks,
                });
            }
        }
        let (state, reason) = if !observations.is_empty() {
            (DistributionActivityState::ObservedInEvaluatedSubset, None)
        } else if refs.is_empty() {
            (
                DistributionActivityState::Unavailable,
                Some(DistributionActivityReason::NoUniqueParameterRef),
            )
        } else {
            (
                DistributionActivityState::Unavailable,
                Some(DistributionActivityReason::NotObservedInEvaluatedSubset),
            )
        };
        DistributionCandidateActivity {
            state,
            reason,
            observations,
        }
    }

    fn dependency_context(
        &self,
        app: DistributionSourceRef,
        refs: &[usize],
    ) -> Result<DistributionCandidateDependencies> {
        let mut declarations = Vec::new();
        for &r in refs {
            if let Some(found) = self.dependencies.get(&self.doc.coordinate(r)?) {
                declarations.extend(found.iter().copied());
            }
        }
        Ok(DistributionCandidateDependencies {
            impact_scope: if self.opaque.contains(&app) {
                DistributionImpactScope::ApplicationImpactUnknown
            } else {
                DistributionImpactScope::DeclaredEndpointsOnly
            },
            declarations,
        })
    }

    fn value_check(
        &self,
        owner: usize,
        shape: std::result::Result<usize, DistributionTypeShape>,
    ) -> DistributionValueCheck {
        use DistributionSideComparison as Side;
        use DistributionValueComparison as Cmp;
        let doc = self.doc;
        let element = shape.ok();
        let (value, v) = decimal_field(doc, Some(owner), "Value");
        let (minimum, min) = decimal_field(doc, element, "minInclusive");
        let (maximum, max) = decimal_field(doc, element, "maxInclusive");
        let reversed =
            matches!((&min, &max), (Some(a), Some(b)) if a.compare(b) == Ordering::Greater);
        let side = |field: &DistributionDecimalField, bound: &Option<Exact>, outside: Ordering| {
            if element.is_none() {
                return Side::Unavailable;
            }
            if field.class == DistributionDecimalClass::Absent {
                return Side::UnboundedOnThatSide;
            }
            match (&v, bound) {
                (Some(v), Some(b)) if !reversed => {
                    if v.compare(b) == outside {
                        Side::Outside
                    } else {
                        Side::NotOutside
                    }
                }
                _ => Side::Unavailable,
            }
        };
        let below = side(&minimum, &min, Ordering::Less);
        let above = side(&maximum, &max, Ordering::Greater);
        let refused =
            |f: &DistributionDecimalField| f.class == DistributionDecimalClass::ExceedsLexicalLimit;
        let comparison = if element.is_none() {
            Cmp::TypeUnavailable
        } else {
            match value.class {
                DistributionDecimalClass::Absent => Cmp::ValueAbsent,
                DistributionDecimalClass::Empty => Cmp::ValueEmpty,
                DistributionDecimalClass::NotFiniteDecimal => Cmp::NotFiniteDecimal,
                DistributionDecimalClass::ExceedsLexicalLimit => Cmp::LexicalResourceLimit,
                DistributionDecimalClass::FiniteDecimal => {
                    if reversed {
                        Cmp::InvalidDeclaredRange
                    } else if below == Side::Outside {
                        Cmp::Below
                    } else if above == Side::Outside {
                        Cmp::Above
                    } else if (below == Side::Unavailable && refused(&minimum))
                        || (above == Side::Unavailable && refused(&maximum))
                    {
                        Cmp::LexicalResourceLimit
                    } else if below == Side::Unavailable || above == Side::Unavailable {
                        Cmp::BoundInvalid
                    } else if below == Side::UnboundedOnThatSide
                        && above == Side::UnboundedOnThatSide
                    {
                        Cmp::Unbounded
                    } else {
                        Cmp::Within
                    }
                }
            }
        };
        DistributionValueCheck {
            model: DistributionArithmeticModel::FiniteDecimalComparisonV1,
            type_shape: match shape {
                Ok(_) => DistributionTypeShape::TypeNumber,
                Err(shape) => shape,
            },
            type_source: element.and_then(|e| doc.coordinate(e).ok()),
            type_attributes: element
                .map(|e| {
                    ["Type", "SizeInBit"]
                        .iter()
                        .map(|&name| {
                            let value = doc.nodes[e].attr(name);
                            DistributionDeclaredAttribute {
                                name: name.into(),
                                presence: presence(&value),
                                value,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
            value,
            minimum,
            maximum,
            below_minimum: below,
            above_maximum: above,
            comparison,
        }
    }
}

/// The Union of a member Parameter: `Static/Parameters/Union/Parameter` in the document namespace.
fn union_of(doc: &Document<'_>, member: usize) -> Option<usize> {
    let union = doc.nodes[member].parent?;
    let parameters = doc.nodes[union].parent?;
    (named(doc, member, "Parameter")
        && named(doc, union, "Union")
        && named(doc, parameters, "Parameters")
        && doc.semantic_node(union)
        && doc.declaration(member))
    .then_some(union)
}

/// `union-member-relative-bits-v1`, strict-explicit: an omitted coordinate yields no interval.
fn interval_check(
    doc: &Document<'_>,
    union: usize,
    member: usize,
    typing: &ParameterTyping,
) -> Result<DistributionIntervalCheck> {
    use DistributionIntegerClass as C;
    use DistributionIntervalComparison as I;
    let element = typing.element.ok();
    let offset = integer_field(doc, Some(member), "Offset");
    let bit_offset = integer_field(doc, Some(member), "BitOffset");
    let width = integer_field(doc, element, "SizeInBit");
    let extent = integer_field(doc, Some(union), "SizeInBit");
    let coordinate = |f: &DistributionIntegerField, absent, invalid| match f.class {
        C::Integer => None,
        C::Absent => Some(absent),
        C::Empty | C::NotInteger => Some(invalid),
        C::ExceedsLimit => Some(I::ArithmeticLimit),
    };
    let mut problem = coordinate(&offset, I::OffsetAbsent, I::OffsetInvalid)
        .or_else(|| coordinate(&bit_offset, I::BitOffsetAbsent, I::BitOffsetInvalid))
        .or_else(|| match (width.class, width.parsed) {
            (C::Integer, Some(w)) if w < 0 => Some(I::WidthInvalid),
            _ => coordinate(&width, I::WidthUnknown, I::WidthInvalid),
        });
    let mut interval = None;
    if problem.is_none() {
        let (o, b, w) = (
            offset.parsed.unwrap_or_default(),
            bit_offset.parsed.unwrap_or_default(),
            width.parsed.unwrap_or_default(),
        );
        match o
            .checked_mul(8)
            .and_then(|s| s.checked_add(b))
            .and_then(|s| Some((s, s.checked_add(w)?)))
        {
            Some((start_bit, end_bit)) => {
                interval = Some(DistributionBitInterval { start_bit, end_bit })
            }
            None => problem = Some(I::ArithmeticLimit),
        }
    }
    let comparison = match (problem, interval) {
        (Some(problem), _) => problem,
        (None, Some(i)) => match (extent.class, extent.parsed) {
            (C::Integer, Some(e)) if e < 0 => I::ExtentInvalid,
            (C::Integer, Some(e)) => {
                if i.start_bit < 0 || i.end_bit > e {
                    I::OutsideExtent
                } else {
                    I::WithinExtent
                }
            }
            (C::Absent, _) => I::ExtentUnknown,
            (C::ExceedsLimit, _) => I::ArithmeticLimit,
            _ => I::ExtentInvalid,
        },
        (None, None) => I::ArithmeticLimit,
    };
    Ok(DistributionIntervalCheck {
        model: DistributionArithmeticModel::UnionMemberRelativeBitsV1,
        union_source: doc.coordinate(union)?,
        union_placement: doc
            .children(union)
            .filter(|&c| named(doc, c, "Memory") || named(doc, c, "Property"))
            .map(|c| doc.coordinate(c))
            .collect::<Result<_>>()?,
        type_source: element.map(|e| doc.coordinate(e)).transpose()?,
        zero_width_declared: width.class == C::Integer && width.parsed == Some(0),
        offset,
        bit_offset,
        width,
        extent,
        interval,
        comparison,
    })
}

fn union_summary(
    doc: &Document<'_>,
    union: usize,
    members: &[DistributionIntervalCheck],
) -> DistributionUnionSummary {
    let mut distinct = BTreeMap::<(i64, i64), u64>::new();
    for i in members.iter().filter_map(|m| m.interval) {
        *distinct.entry((i.start_bit, i.end_bit)).or_default() += 1;
    }
    let distinct_intervals = distinct
        .into_iter()
        .map(
            |((start_bit, end_bit), members)| DistributionDistinctInterval {
                start_bit,
                end_bit,
                members,
            },
        )
        .collect::<Vec<_>>();
    // Sorted by start: a later non-empty interval overlaps iff it starts before this one ends.
    let nonempty = distinct_intervals
        .iter()
        .enumerate()
        .filter(|(_, d)| d.start_bit < d.end_bit)
        .map(|(i, d)| (i as u32, d.start_bit, d.end_bit))
        .collect::<Vec<_>>();
    let mut pairs = 0u64;
    let mut samples = Vec::new();
    for (k, &(first, _, end)) in nonempty.iter().enumerate() {
        let later = &nonempty[k + 1..];
        let overlapping = later.partition_point(|&(_, start, _)| start < end);
        pairs += overlapping as u64;
        for &(second, _, _) in later.iter().take(overlapping) {
            if samples.len() == MAX_OVERLAP_SAMPLES {
                break;
            }
            samples.push(DistributionIntervalPair { first, second });
        }
    }
    DistributionUnionSummary {
        model: DistributionArithmeticModel::UnionMemberRelativeBitsV1,
        extent: integer_field(doc, Some(union), "SizeInBit"),
        member_declarations: members.len() as u64,
        known_intervals: members.iter().filter(|m| m.interval.is_some()).count() as u64,
        unavailable_intervals: members.iter().filter(|m| m.interval.is_none()).count() as u64,
        unknown_width_members: members
            .iter()
            .filter(|m| m.width.class == DistributionIntegerClass::Absent)
            .count() as u64,
        outside_members: members
            .iter()
            .filter(|m| m.comparison == DistributionIntervalComparison::OutsideExtent)
            .count() as u64,
        overlap_samples_truncated: pairs > samples.len() as u64,
        overlapping_distinct_pairs: pairs,
        overlap_samples: samples,
        distinct_intervals,
    }
}

/// Reports candidates in element order; nothing is corrected, initialized or written.
pub(super) fn inspect(
    doc: &Document<'_>,
    index: &ReferenceIndex,
    references: &mut usize,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<()> {
    let mut refs_by_parameter = HashMap::<usize, Vec<usize>>::new();
    for id in 0..doc.nodes.len() {
        if named(doc, id, "ParameterRef") && doc.declaration(id) {
            if let [one] = lookup(doc, index, id, "RefId", "Parameter").1.as_slice() {
                refs_by_parameter.entry(*one).or_default().push(id);
            }
        }
    }
    let mut evaluated = HashSet::new();
    let mut active = HashMap::<_, Vec<_>>::new();
    for app in &report.applications {
        if app.dynamic_state != DistributionPassState::PartiallyEvaluated {
            continue;
        }
        evaluated.insert(app.source);
        for r in &app.active_parameter_refs {
            let module = r.module_scope.last().map(|s| s.module_def_id.clone());
            active
                .entry((app.source, r.ref_id.clone(), module))
                .or_default()
                .push(r.module_scope.clone());
        }
    }
    let mut dependencies = HashMap::<_, Vec<_>>::new();
    let mut opaque = HashSet::new();
    for e in &report.dependency_evidence {
        if e.impact_scope == DistributionImpactScope::ApplicationImpactUnknown {
            opaque.insert(e.application_source);
        }
        for p in &e.endpoints {
            for &c in &p.candidates {
                dependencies
                    .entry(c)
                    .or_default()
                    .push(DistributionCandidateDependency {
                        kind: e.kind,
                        role: p.role,
                        source: e.source,
                        parameter_ref_source: c,
                    });
            }
        }
    }
    let mut pass = Pass {
        doc,
        references,
        max_references: limits.max_references,
        refs_by_parameter,
        evaluated,
        active,
        dependencies,
        opaque,
    };
    let no_refs = Vec::new();
    for id in 0..doc.nodes.len() {
        // The name filter runs first, so ancestor walks are paid only by candidate elements.
        if !["Union", "Parameter", "ParameterRef"]
            .iter()
            .any(|&local| named(doc, id, local))
        {
            continue;
        }
        let (Some(app), Some(manufacturer)) = (doc.application(id), doc.manufacturer(id)) else {
            continue;
        };
        let app_source = doc.coordinate(app)?;
        let scope = doc.entity_scope(id).filter(|&s| s != app);
        let row =
            |kind, pass: &Pass<'_, '_>, refs: &[usize]| -> Result<DistributionSourceCandidate> {
                Ok(DistributionSourceCandidate {
                    kind,
                    severity: DistributionCandidateSeverity::SourceCandidate,
                    source: doc.coordinate(id)?,
                    application_source: app_source,
                    manufacturer_source: doc.coordinate(manufacturer)?,
                    module_definition_source: scope.map(|s| doc.coordinate(s)).transpose()?,
                    parameter: None,
                    parameter_type: None,
                    value_check: None,
                    interval_check: None,
                    union_summary: None,
                    activity: pass.activity(app_source, refs),
                    initialization: DistributionCandidateInitialization::NotPerformed,
                    dependency_context: pass.dependency_context(app_source, refs)?,
                })
            };
        let mut rows = Vec::new();
        if named(doc, id, "Union") && doc.semantic_node(id) {
            let members = doc
                .children(id)
                .filter(|&m| union_of(doc, m) == Some(id))
                .map(|m| interval_check(doc, id, m, &typing(doc, index, m)))
                .collect::<Result<Vec<_>>>()?;
            if !members.is_empty() {
                let mut summary = row(Kind::UnionIntervalSummary, &pass, &[])?;
                summary.activity = DistributionCandidateActivity {
                    state: DistributionActivityState::Unavailable,
                    reason: Some(DistributionActivityReason::SummaryRow),
                    observations: Vec::new(),
                };
                summary.union_summary = Some(union_summary(doc, id, &members));
                rows.push(summary);
            }
        } else if named(doc, id, "Parameter") && doc.declaration(id) {
            let typing = typing(doc, index, id);
            let refs = pass.refs_by_parameter.get(&id).unwrap_or(&no_refs).clone();
            if typing.eligible(doc) {
                let mut value = row(Kind::ParameterValue, &pass, &refs)?;
                value.parameter_type = Some(pass.type_target(&typing)?);
                value.value_check = Some(pass.value_check(id, typing.element));
                rows.push(value);
            }
            if let Some(union) = union_of(doc, id) {
                let mut member = row(Kind::UnionMemberInterval, &pass, &refs)?;
                member.parameter_type = Some(pass.type_target(&typing)?);
                member.interval_check = Some(interval_check(doc, union, id, &typing)?);
                rows.push(member);
            }
        } else if named(doc, id, "ParameterRef")
            && doc.declaration(id)
            && doc.nodes[id].attr("Value").is_some()
        {
            let (raw, parameters, basis) = lookup(doc, index, id, "RefId", "Parameter");
            let typing = match parameters.as_slice() {
                [one] => Some(typing(doc, index, *one)),
                _ => None,
            };
            if typing.as_ref().is_none_or(|t| t.eligible(doc)) {
                let mut value = row(Kind::ParameterRefValue, &pass, &[id])?;
                value.parameter = Some(pass.target("RefId", (raw, &parameters, basis))?);
                let shape = match &typing {
                    Some(t) => {
                        value.parameter_type = Some(pass.type_target(t)?);
                        t.element
                    }
                    None => Err(DistributionTypeShape::ParameterUnavailable),
                };
                value.value_check = Some(pass.value_check(id, shape));
                rows.push(value);
            }
        }
        for row in rows {
            doc.push(&mut report.source_candidates, row)?;
            if report.source_candidates.len() > limits.max_output_records {
                return Err(limit("output records"));
            }
        }
    }
    Ok(())
}
