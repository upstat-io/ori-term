---
schema_version: "0.1-provisional"
---

# Catalog Line-Number-Primary Citation Fixture

Row that MUST be rejected: the `Implementation` column uses the
banned line-number-primary form (`file.rs:91 → Symbol`). The plan's
canonical form is symbol-primary with the file path in parentheses.

Expected: `LineNumberPrimaryCitation` finding.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| ECMA48-LN-PRIMARY | ECMA-48 §8.3.21 | `` `CSI Ps;Ps H` `` | Cursor Position (bad cite) | `` `crates/vte/src/ansi/dispatch/csi.rs:91 → TermHandler::goto` `` | state-snapshot | parser:pending | stub | — | Line-number-primary anti-drift violation. |
