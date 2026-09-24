# ADR-0036: Scheme-21 package XML fails closed on foreign namespaces

**Status:** Accepted (2026-09-24)

## Context

The standalone product-package installer admits the exact master namespace
`http://knx.org/xml/project/21`. Its typed master, hardware, catalogue, and
application-program readers dispatch by local XML names, not expanded names.
Thus a foreign-namespace element with a modeled local name can otherwise
create a KNX typed row. A qualified attribute can likewise be mistaken for an
unqualified KNX field. An unknown-evidence reconciliation pass cannot undo a
row already written by a typed reader.

The opt-in read-only corpus contains three observed scheme-21 packages and 13
XML members. Their element namespaces are the canonical scheme-21 URI and their
attributes are unqualified. This observation does not establish the complete
scheme-21 grammar or ETS compatibility.

## Decision

For scheme-21 standalone packages, before publishing any rows, validate each
member sent to the typed master/catalogue/hardware/application-program readers
or the baggage-index parser:
all XML elements must resolve to the exact scheme-21 namespace; attributes must
be unqualified except namespace declarations. Reject a mismatch with a named
XML import error, atomically. Do not silently coerce an extension into a KNX
row or claim extension support. Schemes 11–14 and 20 retain their existing
compatibility boundary. Project-schema-21 `.knxproj` import is a different
adapter and is not changed by this decision.

The streaming evidence pass reports the observed scheme-21 semantic attributes
and retains the original member bytes. It does **not** execute load procedures,
enforce access policies, or implement RF/coupler commissioning.

## Consequences

A valid but unobserved extension-bearing scheme-21 package is refused instead
of partially installed. Future support requires namespace-aware typed readers
and independent fixtures/corpus evidence before this gate can be relaxed. The
preflight rereads parsed XML members within the existing ZIP member/expanded-size
limits; integrity takes priority over avoiding this extra pass.

Synthetic negative cases cover foreign element namespaces and transaction
rollback. The private matrix checks isolated and shared-order installations
without publishing package identities or manufacturer data.
