//! The executor error stays compact while private evidence never changes diagnostic text.
use super::*;
use crate::commissioning::pointer_width::PointerWidthRefusalReason;

#[test]
fn allocation_evidence_keeps_the_executor_error_within_the_result_size_budget() {
    let size = std::mem::size_of::<PropertySimulationError>();
    println!(
        "error={size}, failure={}, pointer={}, session={}",
        std::mem::size_of::<PropertySimulationFailure>(),
        std::mem::size_of::<PointerWidthRefusal>(),
        std::mem::size_of::<SessionError>()
    );
    assert!(
        size < 128,
        "compact executor error, without lint exceptions: {size}"
    );
}

#[test]
fn allocation_evidence_and_private_pointer_text_keep_exact_debug_and_display() {
    let error = PropertySimulationError {
        source_index: Some(4),
        operation_index: Some(5),
        mutation_may_have_started: true,
        completed: vec![],
        failure: PropertySimulationFailure::PointerWidth(PointerWidthRefusal {
            reason: PointerWidthRefusalReason::UnsupportedWidth,
            object: 3,
            property_id: 7,
            elements: 1,
            octets: 3,
            diagnostic: "private pointer 012345".into(),
        }),
        final_step: None,
        final_step_evidence: Some(Box::new(AllocationMismatchEvidence {
            allocation_index: 0,
            object: 3,
            address: 0x3c00,
            prestate_unreadable: false,
            expected: vec![0xde, 0xad],
            actual: vec![0xbe, 0xef],
            written: vec![true; 2],
            differing: vec![],
        })),
    };
    assert_eq!(format!("{error:?}"), "PropertySimulationError { source_index: Some(4), operation_index: Some(5), mutation_may_have_started: true, completed: [], failure: PointerWidth(PointerWidthRefusal { reason: UnsupportedWidth, object: 3, property_id: 7, elements: 1, octets: 3, diagnostic: \"private pointer 012345\" }), final_step: None }");
    assert_eq!(error.to_string(), "property execution stopped at source Some(4): PointerWidth(PointerWidthRefusal { reason: UnsupportedWidth, object: 3, property_id: 7, elements: 1, octets: 3, diagnostic: \"private pointer 012345\" })");
    let PropertySimulationFailure::PointerWidth(refusal) = &error.failure else {
        panic!("typed pointer failure preserved")
    };
    assert_eq!(refusal.private_diagnostic(), "private pointer 012345");
}

#[test]
fn session_failure_keeps_exact_diagnostic_text_and_error_source_type() {
    let error = PropertySimulationError {
        source_index: None,
        operation_index: None,
        mutation_may_have_started: false,
        completed: vec![],
        failure: PropertySimulationFailure::Session(
            SessionError::MemoryRefused { address: 15360 }.into(),
        ),
        final_step: None,
        final_step_evidence: None,
    };
    assert_eq!(format!("{error:?}"), "PropertySimulationError { source_index: None, operation_index: None, mutation_may_have_started: false, completed: [], failure: Session(MemoryRefused { address: 15360 }), final_step: None }");
    assert_eq!(
        error.to_string(),
        "property execution stopped at source None: Session(MemoryRefused { address: 15360 })"
    );
    assert!(matches!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<SessionError>(),
        Some(SessionError::MemoryRefused { address: 15360 })
    ));
}
