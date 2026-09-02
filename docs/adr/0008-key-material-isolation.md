# ADR 0008: Key material isolation

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

KNX Data Secure involves runtime keys, tool keys and factory default setup keys
(RESEARCH §9). All of them are secrets.

None are present in the sample installation, so everything we know about how
ETS stores and uses them is documented rather than verified. That is an
additional reason to build the boundary before the feature, while the cost of
the boundary is zero.

## Decision

`knx-secure` is a separate crate with its own storage, created now, before any
KNX Secure feature exists.

Key material never enters the project model, an import report, an export, a log
or a default diagnostic dump. `knx-secure` does not depend on `knx-core`, so
there is no type path along which a key can reach the project model.

## Alternatives considered

**Add key handling when KNX Secure is implemented.** Rejected because
retrofitting isolation is how secrets leak: by the time the feature exists, key
material would already be flowing through convenient shared types.

## Consequences

An almost empty crate is carried for several sessions. That is the price of the
boundary and it is small.

A test must assert that `knx-secure` types cannot be serialised toward report
or export paths, so the isolation is checked rather than assumed.
