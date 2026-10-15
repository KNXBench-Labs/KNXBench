# Inherited product-corpus prerequisites for P4

Owner authorized fixes after both exact failures reproduced on unmodified
bee4f016. Own branch fix/p4-corpus-prerequisites-20261015 changes tests/docs only.

Matrix cause: ADR0126/9d097e9c models TextParameterRefId; v22 pins never updated.
Instrumented private v22 (1c7016c1, passes) and v23 (bee4f016, fails old pin)
observations differ in exactly five aggregate leaves. All41 other table counts
and all115 public package outcomes match. Independent XML census over115
instances/113 unique packages/302 program blobs predicts reductions465/463/
463/157 at report/per-blob/per-package snapshot granularity, including ModuleDef
scope and nested bundles. Initial incomplete Python scope is retained, corrected
not hidden. Both original probe source trees restored exactly after observation.

Nesting cause: entire loose corpus now includes an archive exceeding standard
expanded-total budget; test unwrapped its typed refusal. New test-only outcome
helper counts only independently matched STANDARD source/member/total size
refusals and demands zero row changes. Other errors remain failing. Reports
aggregate counts, no private names.105=103 installs+1 duplicate+1 refusal;
1678 Module rows/0 nested in admitted subset, not evidence about refused bytes.
Independent ZIP central-directory census predicts104 within-size/one refusal.

RED: original baseline fails both named private cases; new synthetic classifier
fails on standard-size refusal under original semantics. GREEN: all16 explicit
product opt-ins,synthetic boundaries,unrelated metadata-refusal regression and
Clippy. Separate in-session review,not independent approval. All seven semantic mutants fail their named behavioral assertions, sources
restore byte-exactly, and ordinary product tests pass. Final integrated full acceptance,push/readback
and compact evidence cleanup remain pending. No production safety bound changed.
