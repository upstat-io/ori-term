---
schema_version: "1.0"
stack: kittygfx
title: "Kitty Graphics Protocol Catalog"
owner_section: "01 (bootstrap), 13 (verification)"
---

# Kitty Graphics Protocol Catalog

APC `G` commands: transmit / place / delete / animate / query / frame composition. Section 13 (Kitty Graphics) is blocked until the `kitty.rs` BLOAT split lands (see `plans/bug-tracker/section-08-core-terminal.md`).

Dispatch entry: `Performer::apc_start` + `apc_put` + `apc_end` (`crates/vte/src/ansi/dispatch/mod.rs`) → `Term::apc_dispatch` → `Term::handle_apc_dispatch` (`oriterm_core/src/term/handler/image/mod.rs`) dispatches on payload first byte (`G` = Kitty) → `Term::handle_kitty_graphics` (`oriterm_core/src/term/handler/image/kitty.rs`).

Actions are dispatched via `KittyAction` enum (parsed by `parse_kitty_command` in `oriterm_core/src/image/kitty.rs`) and routed to per-action handlers in `kitty.rs` and `kitty_animation.rs`.

## Core Actions

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| KG-QUERY | Kitty graphics-protocol | `` `APC G a=q,i=<id>...; ST` `` | Query (a=q) — respond with OK, no state mutation | `` `Term::handle_kitty_graphics` → `Term::kitty_query` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | effect-pty-write | parser:pending dispatch:pending effect:pending | stub | wezterm escape-sequences.md | Currently hardcoded to respond `OK`; does not validate payload correctness. Memory audit 2026-04-07 previously claimed "NOT IMPLEMENTED" — contradicted by this row (see 01.9 stale-claim correction). |
| KG-TRANSMIT | Kitty graphics-protocol | `` `APC G a=t,f=<fmt>,...; <data> ST` `` | Transmit (a=t) — upload image bytes (RGB, RGBA, or PNG) | `` `Term::kitty_transmit` → `Term::kitty_store_image` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending | implemented-unverified | — | Supports chunked uploads via `more_data` + `kitty_accumulate_chunk`. |
| KG-TRANSMIT-PLACE | Kitty graphics-protocol | `` `APC G a=T,f=<fmt>,...; <data> ST` `` | Transmit and place (a=T) — upload + immediate placement | `` `Term::kitty_transmit_and_place` → `Term::kitty_store_image` + `Term::kitty_create_placement` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending | implemented-unverified | — | |
| KG-PLACE | Kitty graphics-protocol | `` `APC G a=p,i=<id>,p=<pid>,...; ST` `` | Place (a=p) — create placement for existing image | `` `Term::kitty_place` → `Term::kitty_create_placement` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending | implemented-unverified | — | U=1 defers placement to Unicode placeholder cells. |
| KG-DELETE | Kitty graphics-protocol | `` `APC G a=d,d=<spec>,...; ST` `` | Delete (a=d) — delete by specifier (a/A/i/I/p/P/c/C/x/X/y/Y/z/Z/r/R/n/N) | `` `Term::kitty_delete` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | state-snapshot | parser:pending dispatch:pending state:pending | stub | — | **BUG-08-7 (high)**: d=a, d=c, d=p, d=r specifiers diverge from protocol spec. d=n / d=N not implemented. Section 13 drives this to `verified`. |
| KG-FRAME | Kitty graphics-protocol (animation) | `` `APC G a=f,i=<id>,...; <data> ST` `` | Frame transmit (a=f) — append frame to animated image | `` `Term::kitty_frame` (`oriterm_core/src/term/handler/image/kitty_animation.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending | stub | — | Animation support is scaffolded; full correctness is Section 13 work. |
| KG-ANIMATE | Kitty graphics-protocol (animation) | `` `APC G a=a,i=<id>,...; ST` `` | Animate (a=a) — control animation playback | `` `Term::kitty_animate` (`oriterm_core/src/term/handler/image/kitty_animation.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending | stub | — | |

## Unicode Placeholder Protocol

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| KG-UNICODE-PLACEHOLDER | Kitty graphics-protocol (unicode placeholder) | U+10EEEE placeholder character + diacritic row/column encoding | Place existing image into cells via Unicode placeholder character | MISSING — to be added by Section 13 (Kitty Graphics) | frame-input | parser:pending dispatch:pending snapshot:pending | missing | — | U=1 transmit/place arm currently stores the placement deferral flag; cell-level placeholder rendering is TBD. |

## Response Protocol

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| KG-RESPONSE | Kitty graphics-protocol (response) | `` `APC G i=<id>,q=<quiet>;<msg> ST` `` | Response emitted back to PTY on OK / EINVAL / ENOENT / etc. | `` `Term::kitty_respond` (`oriterm_core/src/term/handler/image/kitty.rs`) `` | effect-pty-write | parser:pending dispatch:pending effect:pending | implemented-unverified | — | Honors `q=1` / `q=2` quiet levels. |
