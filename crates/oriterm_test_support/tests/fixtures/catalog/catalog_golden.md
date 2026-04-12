---
schema_version: "0.1-provisional"
---

# Catalog Golden Fixture

Baseline well-formed row covering every column in the 10-column schema.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| ECMA48-GOLD | ECMA-48 §8.3.21 | `` `CSI Ps;Ps H` `` | Cursor Position | `` `TermHandler::goto` (`oriterm_core/src/term/handler/mod.rs`) `` | state-snapshot | parser:pending dispatch:pending state:pending | implemented-unverified | — | Well-formed reference row. |
