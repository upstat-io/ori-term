---
section: "03"
title: "Effect Boundary Migration"
status: not-started
reviewed: false
goal: "Introduce `oriterm_core::effect::{Effect, EffectSink}` as the production interface for boundary-crossing side effects, remove closures from `Event::ClipboardLoad`/`ColorRequest`, absorb `Term::pending_notifications` into the Effect channel, and migrate all current Event consumers via a one-phase `LegacyEventSink` adapter."
success_criteria:
  - "`oriterm_core::effect::{Effect, EffectSink}` exists with the family enum: `Pty(PtyEffect) | Host(HostEffect) | HostRequest(HostRequest) | Ui(UiEffect) | Presentation(PresentationEffect)`"
  - "Closures REMOVED from emission sites: `Event::ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Send + Sync>)` and `Event::ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Send + Sync>)` are no longer constructed in any VTE handler file. The emission sites emit `HostRequest::ClipboardLoad { clipboard_type, reply: ResponseToken }` / `HostRequest::ColorQuery { index, reply: ResponseToken }` instead."
  - "The legacy `Event::ClipboardLoad` and `Event::ColorRequest` variants REMAIN in `oriterm_core/src/event/mod.rs` as **deprecated** shims emitted ONLY by the `LegacyEventSink` adapter (`oriterm_core/src/effect/sink/legacy.rs`) for a one-phase migration. The concrete follow-up plan directory `plans/effect-cutover/` (filed as an in-scope artifact of this section — see 03.N) migrates the last legacy consumers directly to `Effect::HostRequest` and deletes the deprecated variants entirely. This is NOT a contradiction with the 'no closures at emission sites' criterion above — the closures live only in the adapter shim, wrapping a `ResponseToken` for back-compat."
  - "`grep -rn 'Arc<dyn Fn\\|Arc::new(move' oriterm_core/src/term/handler/` returns zero matches (handler files are closure-free; the adapter shim in `oriterm_core/src/effect/sink/legacy.rs` is allowed to use closures until the deprecated Event variants are deleted)"
  - "`Term::pending_notifications` is migrated: notifications flow through `EffectSink::push(Effect::Host(HostEffect::DesktopNotification {...}))` and consumers drain via `EffectSink::take_pending() -> Vec<Effect>`. The legacy `drain_notifications()` method is removed (or becomes a thin shim that calls `take_pending()` and filters)."
  - "`LegacyEventSink` adapter exists in `oriterm_core/src/effect/legacy.rs` that converts `Effect` → existing `Event`/`MuxEvent` so all existing consumers (oriterm_mux, oriterm) keep working during the migration phase"
  - "All VTE handler emission points in `oriterm_core/src/term/handler/{mod,osc,modes,dcs,status}.rs` and `oriterm_core/src/term/handler/image/kitty.rs:465` emit through `EffectSink` (directly or via the legacy adapter)"
  - "Snapshot seqno (`snapshot_seqno: u64` field added to `RenderableContent` or sibling) increments on each successful snapshot publication and is observable from tests — required for sections 04 (harness apex) and 06 (Mode 2026 timeout)"
  - "All existing tests in `oriterm_core/tests/teseq/`, `oriterm_core/tests/tack/`, and the alloc/RSS regression tests pass without modification (LegacyEventSink preserves observable behavior)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Effect/State separation enforced** mission criterion"
inspired_by:
  - "Alacritty `alacritty/alacritty_terminal/src/event.rs` — production event enum that tests observe directly (no test-only parallel interface)"
  - "ori_term existing `Event` enum at `oriterm_core/src/event/mod.rs` — the type being migrated"
depends_on: ["02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Define Effect type family in oriterm_core::effect"
    status: not-started
  - id: "03.2"
    title: "Implement EffectSink trait + concrete bulk-drain implementation"
    status: not-started
  - id: "03.3"
    title: "Add snapshot_seqno field for verification chain harness apex"
    status: not-started
  - id: "03.4"
    title: "LegacyEventSink adapter — bridge Effect to existing Event/MuxEvent consumers"
    status: not-started
  - id: "03.5"
    title: "Migrate VTE handler emission sites to emit Effect"
    status: not-started
  - id: "03.6"
    title: "Migrate Term::pending_notifications into the Effect channel"
    status: not-started
  - id: "03.7"
    title: "Remove ClipboardLoad/ColorRequest closure variants from Event"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 03.4 (after the bridge is built — covers .1-.4),
# 03.6 (after notifications migration — covers .5-.6), final in 03.N
---

# Section 03: Effect Boundary Migration

**Status:** Not Started
**Goal:** Replace ori_term's current `Event` enum with a properly-structured `Effect` family enum that lives in production at `oriterm_core::effect::*`. The migration removes closures from `ClipboardLoad`/`ColorRequest` (replaced with typed request/response via `ResponseToken`), absorbs `Term::pending_notifications` (the bypass channel) into the Effect drain, separates fire-and-forget host effects from request/response patterns, and adds the `snapshot_seqno` counter that section 04's verification chain harness will observe as the Mode 2026 commit apex. Migration is one-phase via a `LegacyEventSink` adapter — no big-bang refactor, no observable behavior change in existing tests.

**Success Criteria:**
- [ ] `oriterm_core::effect::Effect` family enum exists with all 5 sub-families
- [ ] No closures in any `Event` variant — `grep -rn 'Arc<dyn Fn' oriterm_core/src/event/ oriterm_core/src/term/handler/osc.rs` returns zero
- [ ] `Term::pending_notifications` bypass is gone — notifications flow through `EffectSink::push(Effect::Host(HostEffect::DesktopNotification {...}))`
- [ ] `LegacyEventSink` adapter exists and bridges Effect to existing consumers
- [ ] `snapshot_seqno` increments on snapshot publication and is observable
- [ ] All existing tests pass without modification (`./test-all.sh` green debug + release)
- [ ] No regressions in `oriterm_core/tests/alloc_regression.rs` (closure removal must not introduce new allocations on hot paths)
- [ ] Connects to mission criterion: **Effect/State separation enforced**

**Context:** Codex's Round 2 + Round 3 consensus established that the current `Event` enum mixes four different abstractions (state changes, fire-and-forget effects, request/response with closures, transport noise like Wakeup). The closure-based `ClipboardLoad` and `ColorRequest` carry `Arc<dyn Fn(...) -> String>` payloads that capture formatter state from the OSC handler and pass it to the consumer — this is awkward, leaks formatting logic out of `oriterm_core`, and prevents tests from cleanly observing what response the handler will format. The fix is to switch to typed request/response: the handler emits `HostRequest::ClipboardLoad { sel, reply: token }`, the consumer satisfies the request and delivers the reply via the token, and the terminal then formats the reply via its own `Effect::Pty(PtyEffect::Write(...))` emission. Pass 1 + Pass 2 confirmed the exact closure signatures at `oriterm_core/src/event/mod.rs:46,50` and the bypass channel at `oriterm_core/src/term/shell_state.rs:218`.

**Reference implementations:**
- **Alacritty** `alacritty/alacritty_terminal/src/event.rs` — production event enum that tests observe directly. No test-only parallel interface; same type production and tests share, eliminating drift risk. Pattern ori_term should follow.
- **ori_term existing** `oriterm_core/src/event/mod.rs:27-63` — current Event enum with closures (the migration target)
- **ori_term existing** `oriterm_core/src/term/handler/osc.rs:102-110, 138-159` — current closure emission sites
- **ori_term existing** `oriterm_core/src/term/shell_state.rs:217-225` — `pending_notifications` bypass channel

**Depends on:** Section 02 (tack-conformance absorption complete; this section's effect type changes propagate through to tack-conformance test files via the LegacyEventSink, but only after section 02 has marked tack-conformance superseded so the test file changes are scoped under spec-conformance).

---

## 03.1 Define Effect type family in oriterm_core::effect

**File(s):** `oriterm_core/src/effect/mod.rs` (new), `oriterm_core/src/effect/effect.rs` (new), `oriterm_core/src/effect/families/{pty,host,host_request,ui,presentation}.rs` (new), `oriterm_core/src/effect/tests.rs` (new)

Create the new `effect` module with the family enum and sub-types. Pure type definitions; no behavior. Per CLAUDE.md test organization rules, source code in `mod.rs` + leaf files, tests in sibling `tests.rs`.

- [ ] Create `oriterm_core/src/effect/mod.rs` as the dispatch hub:
  ```rust
  //! Boundary-crossing terminal side effects.
  //!
  //! `Effect` is the production interface for everything that LEAVES the terminal:
  //! PTY writes, host platform calls (clipboard, audio, title, notification),
  //! UI hints, and presentation gates (sync output). State changes (cells,
  //! cursor, palette, modes, image placements, hyperlinks) are NOT effects —
  //! they are observed via `RenderableContent` snapshots.
  //!
  //! See `plans/spec-conformance/00-overview.md` for the architectural rationale.

  mod effect;
  mod families;
  mod sink;

  pub use effect::Effect;
  pub use families::{
      AudioRequest, AudioKind, ClipboardSelection, HostEffect, HostRequest,
      PresentationEffect, PtyEffect, ResponseToken, SyncAbortReason, UiEffect,
  };
  pub use sink::{EffectSink, LegacyEventSink};

  #[cfg(test)]
  mod tests;
  ```
- [ ] Create `oriterm_core/src/effect/effect.rs`:
  ```rust
  use super::families::*;

  /// Top-level effect family routed via the EffectSink.
  ///
  /// The five families partition by purpose: Pty for PTY writes, Host for
  /// fire-and-forget platform calls, HostRequest for typed request/response
  /// (NOT closures), Ui for UI hints, Presentation for sync gates.
  #[derive(Debug, Clone)]
  pub enum Effect {
      Pty(PtyEffect),
      Host(HostEffect),
      HostRequest(HostRequest),
      Ui(UiEffect),
      Presentation(PresentationEffect),
  }
  ```
- [ ] Create `oriterm_core/src/effect/families/mod.rs` + per-family files:
  ```rust
  mod pty;
  mod host;
  mod host_request;
  mod ui;
  mod presentation;

  pub use pty::{PtyEffect, PtyWriteKind};
  pub use host::{
      HostEffect, ClipboardSelection, AudioRequest, AudioKind,
      PrintRequest, PrintKind, NotificationSource,
  };
  pub use host_request::{HostRequest, ResponseToken, ResponseFulfilled};
  pub use ui::UiEffect;
  pub use presentation::{PresentationEffect, SyncAbortReason};
  ```
- [ ] Implement each family file. `pty.rs`:
  ```rust
  #[derive(Debug, Clone)]
  pub enum PtyEffect {
      /// Write bytes back to the PTY (DA1/DA2/DA3 reply, CPR, DSR, DECRPM,
      /// DECRQSS reply, kitty image protocol ACK/error, mouse-encoded events,
      /// keyboard-encoded events, focus events, etc.)
      Write { bytes: Vec<u8>, kind: PtyWriteKind },
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum PtyWriteKind {
      DeviceAttribute,
      CursorReport,
      DeviceStatus,
      ModeReport,
      StatusString,
      ImageProtocolReply,
      MouseEvent,
      KeyboardEvent,
      FocusEvent,
      Other,
  }
  ```
- [ ] Implement `host.rs` (fire-and-forget effects):
  ```rust
  #[derive(Debug, Clone)]
  pub enum HostEffect {
      Bell,
      /// Visual bell (DECVB, mode 12) — separate variant, not a flag on Bell.
      /// Rationale: audible and visual bells route to different host consumers
      /// (audio adapter vs UI flash animator); forcing every consumer to
      /// check a flag is worse than dispatching on the variant directly.
      ///
      VisualBell,
      DesktopNotification {
          source: NotificationSource,
          title: String,
          body: String,
      },
      TitleSet { value: Option<String> },
      IconNameSet { value: Option<String> },
      CwdSet { cwd: String },
      AudioRequest(AudioRequest),
      PrintRequest(PrintRequest),
      ChildExit { code: i32 },
      CommandComplete { duration: std::time::Duration },
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum NotificationSource { Osc9, Osc99, Osc777 }

  #[derive(Debug, Clone)]
  pub struct AudioRequest { pub kind: AudioKind, pub params: AudioParams }
  // ... AudioKind, AudioParams, ClipboardSelection, etc.
  ```
- [ ] Implement `host_request.rs` (typed request/response — replaces closures):
  ```rust
  use std::sync::{Arc, Mutex};

  #[derive(Debug, Clone)]
  pub enum HostRequest {
      /// Replaces Event::ClipboardLoad — was: `Arc<dyn Fn(&str) -> String + Send + Sync>` closure.
      ClipboardLoad {
          selection: super::ClipboardSelection,
          reply: ResponseToken<String>,
      },
      /// Replaces Event::ColorRequest — was: `Arc<dyn Fn(Rgb) -> String + Send + Sync>` closure.
      ColorQuery {
          index: u16,
          reply: ResponseToken<crate::color::Rgb>,
      },
      ClipboardStore {
          selection: super::ClipboardSelection,
          data: String,
      },
  }

  /// Token the consumer holds to deliver a reply to a HostRequest.
  ///
  /// The terminal handler creates a ResponseToken when emitting the request,
  /// and the consumer fulfills the request by calling `token.fulfill(value)`.
  /// The terminal then observes the fulfillment via `take_response()` and
  /// formats the reply for PTY emission via `EffectSink::push(Effect::Pty(...))`.
  ///
  /// Implementation note: a `ResponseToken<T>` wraps an `Arc<Mutex<Option<T>>>`
  /// — the consumer puts the response into the slot; the terminal drains the
  /// slot on the next event loop tick. NOT a closure — the value is plain data.
  #[derive(Debug, Clone)]
  pub struct ResponseToken<T> {
      slot: Arc<Mutex<Option<T>>>,
  }

  impl<T> ResponseToken<T> {
      pub fn new() -> Self { Self { slot: Arc::new(Mutex::new(None)) } }
      pub fn fulfill(&self, value: T) { *self.slot.lock().unwrap() = Some(value); }
      pub fn take(&self) -> Option<T> { self.slot.lock().unwrap().take() }
  }

  pub type ResponseFulfilled = ();
  ```
- [ ] Implement `ui.rs`:
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum UiEffect {
      CursorBlinkChanged { enabled: bool },
      MouseCursorDirty,
  }
  ```
- [ ] Implement `presentation.rs` (the Mode 2026 / sync output gate observables):
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum PresentationEffect {
      SyncBegin,
      SyncCommit { snapshot_seqno: u64 },
      SyncAbort { reason: SyncAbortReason },
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum SyncAbortReason {
      Timeout,
      MaxBufferBytesExceeded,
      AppDisconnected,
  }
  ```
- [ ] Add `pub mod effect;` to `oriterm_core/src/lib.rs`.
- [ ] Create `oriterm_core/src/effect/tests.rs` with basic constructibility tests for each variant.
- [ ] **Validation**: `cargo check -p oriterm_core` passes; new types compile without breaking the existing build.

---

## 03.2 Implement EffectSink trait + concrete bulk-drain implementation

**File(s):** `oriterm_core/src/effect/sink.rs` (new), `oriterm_core/src/effect/sink/tests.rs` (new)

The `EffectSink` trait is the production-side interface that the VTE handler emits to. The default concrete implementation is a thread-safe queue that accumulates effects and is drained in bulk by the consumer.

- [ ] Create `oriterm_core/src/effect/sink/mod.rs`:
  ```rust
  use std::sync::{Arc, Mutex};
  use super::Effect;

  /// Receives terminal effects from the VTE handler.
  ///
  /// Default implementation is a thread-safe queue (`QueueingEffectSink`) that
  /// accumulates effects and is drained in bulk via `take_pending()`. The
  /// consumer (mux event proxy, app event loop) calls `take_pending()` after
  /// each parse chunk and processes the drained list.
  pub trait EffectSink: Send + Sync {
      /// Push an effect onto the sink. Cheap, lock-free where possible.
      fn push(&self, effect: Effect);

      /// Drain all pending effects. Called by the consumer once per event loop tick.
      ///
      /// Returns an empty Vec when no effects are pending — caller should treat
      /// the empty case as the common path (no allocation if none queued).
      fn take_pending(&self) -> Vec<Effect>;
  }

  /// Default thread-safe queue-backed sink.
  #[derive(Debug, Default, Clone)]
  pub struct QueueingEffectSink {
      queue: Arc<Mutex<Vec<Effect>>>,
  }

  impl QueueingEffectSink {
      pub fn new() -> Self { Self::default() }
  }

  impl EffectSink for QueueingEffectSink {
      fn push(&self, effect: Effect) {
          self.queue.lock().unwrap().push(effect);
      }
      fn take_pending(&self) -> Vec<Effect> {
          let mut q = self.queue.lock().unwrap();
          if q.is_empty() {
              Vec::new()
          } else {
              std::mem::take(&mut *q)
          }
      }
  }

  /// No-op sink used for tests that don't observe effects.
  #[derive(Debug, Default, Clone, Copy)]
  pub struct VoidEffectSink;

  impl EffectSink for VoidEffectSink {
      fn push(&self, _effect: Effect) {}
      fn take_pending(&self) -> Vec<Effect> { Vec::new() }
  }
  ```
- [ ] Add to `oriterm_core/src/effect/sink/legacy.rs`: stub for `LegacyEventSink` (filled in 03.4).
- [ ] Add `pub mod sink;` to `oriterm_core/src/effect/mod.rs`.
- [ ] Sibling tests in `oriterm_core/src/effect/sink/tests.rs`:
  - `queueing_sink_push_take_roundtrip()`
  - `queueing_sink_take_empty_returns_empty_vec_no_alloc()` — uses counting allocator
  - `void_sink_drops_effects_silently()`
- [ ] **Validation**: `cargo test -p oriterm_core --lib effect::sink::tests` passes; alloc regression on `take_empty` is 0.

---

## 03.3 Add snapshot_seqno field for verification chain harness apex

**File(s):** `oriterm_core/src/term/renderable/mod.rs`, `oriterm_core/src/term/snapshot.rs`, `oriterm_mux/src/pane/io_thread/mod.rs`, sibling tests

Section 04's verification chain harness needs to observe a monotonically increasing counter that ticks on every successful snapshot publication. This is the apex for Mode 2026 sync tests (section 06): "the seqno does not advance during sync; it advances atomically on commit."

- [ ] Add `snapshot_seqno: u64` to `RenderableContent` in `oriterm_core/src/term/renderable/mod.rs`. Field is reset to 0 by `RenderableContent::default()`.
- [ ] In `oriterm_core/src/term/snapshot.rs::renderable_content_into()`, increment the counter at the START of the function. The increment is observable to anyone who reads the snapshot.
- [ ] In `oriterm_mux/src/pane/io_thread/mod.rs::produce_snapshot()` (around line 309), the seqno is filled from the term's running counter when the snapshot is published. This ensures consumers reading the published snapshot see the expected seqno.
- [ ] **Important**: the counter MUST NOT increment when a snapshot publication is suppressed by Mode 2026 sync (see io_thread.rs:268-271 — `if sync_bytes_count > 0 { return; }`). The counter's monotonic-tick semantics are: "ticks on every committed publication, never on a suppressed one."
- [ ] Add tests in `oriterm_core/src/term/renderable/tests.rs`:
  - `snapshot_seqno_increments_on_publication()`
  - `snapshot_seqno_unchanged_on_suppressed_publication()` — feeds bytes during sync, asserts seqno doesn't advance
  - `snapshot_seqno_advances_atomically_on_sync_commit()` — feeds bytes, asserts seqno advances by exactly 1 when sync ends
- [ ] **Validation**: tests pass; alloc regression unchanged.
- [ ] **TPR checkpoint** — `/tpr-review` covering 03.1–03.3 (Effect type family + sink + seqno). Catches API design issues before they cascade through the migration.

---

## 03.4 LegacyEventSink adapter — bridge Effect to existing Event/MuxEvent consumers

**File(s):** `oriterm_core/src/effect/sink/legacy.rs`, `oriterm_core/src/effect/sink/legacy/tests.rs`

The migration is one-phase via an adapter: `LegacyEventSink` receives `Effect` pushes from the VTE handler and converts them into the existing `Event` variants that all current consumers (oriterm_mux's `MuxEventProxy`, oriterm's `EventLoopProxy`, etc.) already understand. This means the migration can land in pieces without breaking anything — handlers emit Effect, the legacy adapter routes to existing consumers via Event, and the existing consumers don't need to change yet.

- [ ] Implement `LegacyEventSink` in `oriterm_core/src/effect/sink/legacy.rs`:
  ```rust
  use crate::event::{Event, EventListener};
  use super::{Effect, EffectSink};
  use crate::effect::families::*;

  /// Adapter that translates Effect pushes into legacy Event emissions.
  ///
  /// This is the migration bridge: the VTE handler emits Effect, the legacy
  /// adapter wraps an existing EventListener and forwards each Effect as the
  /// equivalent Event. Existing consumers (oriterm_mux, oriterm) don't need
  /// to change — they keep receiving Events. Section 03 builds this adapter;
  /// future sections gradually migrate consumers to subscribe to Effect
  /// directly, at which point this adapter can be deleted.
  pub struct LegacyEventSink<L: EventListener> {
      listener: L,
  }

  impl<L: EventListener> LegacyEventSink<L> {
      pub fn new(listener: L) -> Self { Self { listener } }
  }

  impl<L: EventListener + Send + Sync> EffectSink for LegacyEventSink<L> {
      fn push(&self, effect: Effect) {
          // Convert each Effect to the equivalent legacy Event and forward.
          let event = match effect {
              Effect::Pty(PtyEffect::Write { bytes, .. }) => {
                  let s = String::from_utf8_lossy(&bytes).into_owned();
                  Event::PtyWrite(s)
              }
              Effect::Host(HostEffect::Bell) => Event::Bell,
              // Visual bell has no legacy Event variant — route via a thin
              // notifier adapter OR drop silently on legacy consumers.
              // Section 20.1 wires the real UI consumer.
              //
              Effect::Host(HostEffect::VisualBell) => return,
              Effect::Host(HostEffect::TitleSet { value: Some(t) }) => Event::Title(t),
              Effect::Host(HostEffect::TitleSet { value: None }) => Event::ResetTitle,
              Effect::Host(HostEffect::IconNameSet { value: Some(n) }) => Event::IconName(n),
              Effect::Host(HostEffect::IconNameSet { value: None }) => Event::ResetIconName,
              Effect::Host(HostEffect::CwdSet { cwd }) => Event::Cwd(cwd),
              Effect::Host(HostEffect::CommandComplete { duration }) => Event::CommandComplete(duration),
              Effect::Host(HostEffect::ChildExit { code }) => Event::ChildExit(code),
              // Notifications previously had no Event variant — they were drained
              // separately via Term::pending_notifications. The legacy adapter
              // SUPPRESSES Effect::Host(HostEffect::DesktopNotification {...}) here
              // because section 03.6 will replace the drain channel with the
              // sink itself; the adapter only needs to bridge Events that
              // existing code already consumes.
              Effect::Host(HostEffect::DesktopNotification { .. }) => return,
              Effect::Host(HostEffect::AudioRequest(_)) | Effect::Host(HostEffect::PrintRequest(_)) => return, // not yet wired
              Effect::HostRequest(HostRequest::ClipboardStore { selection, data }) => {
                  Event::ClipboardStore(selection_to_legacy(selection), data)
              }
              Effect::HostRequest(HostRequest::ClipboardLoad { selection, reply }) => {
                  // The adapter forwards as Event::ClipboardLoad with a wrapper closure
                  // that fulfills the response token. This preserves the old API
                  // surface for current consumers but the closure is now a thin
                  // wrapper, not the formatter. Section 05+ migrate consumers to
                  // subscribe to HostRequest directly, at which point this branch
                  // is deleted.
                  let token = reply.clone();
                  Event::ClipboardLoad(
                      selection_to_legacy(selection),
                      std::sync::Arc::new(move |text: &str| {
                          token.fulfill(text.to_string());
                          // Return the legacy formatted reply for back-compat
                          format!("\x1b]52;c;{}\x1b\\", base64_encode(text))
                      }),
                  )
              }
              Effect::HostRequest(HostRequest::ColorQuery { index, reply }) => {
                  let token = reply.clone();
                  Event::ColorRequest(
                      index as usize,
                      std::sync::Arc::new(move |color: crate::color::Rgb| {
                          token.fulfill(color);
                          format!("\x1b]4;{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}\x1b\\",
                              index, color.r, color.r, color.g, color.g, color.b, color.b)
                      }),
                  )
              }
              Effect::Ui(UiEffect::CursorBlinkChanged { .. }) => Event::CursorBlinkingChange,
              Effect::Ui(UiEffect::MouseCursorDirty) => Event::MouseCursorDirty,
              Effect::Presentation(_) => return, // not consumed by legacy Event listeners
          };
          self.listener.send_event(event);
      }

      fn take_pending(&self) -> Vec<Effect> {
          // Legacy adapter is push-only — effects are forwarded immediately as Events.
          // take_pending() returns empty because there is no internal queue.
          Vec::new()
      }
  }

  fn selection_to_legacy(s: ClipboardSelection) -> crate::event::ClipboardType {
      match s {
          ClipboardSelection::Clipboard => crate::event::ClipboardType::Clipboard,
          ClipboardSelection::Selection => crate::event::ClipboardType::Selection,
      }
  }

  fn base64_encode(input: &str) -> String {
      use base64::{Engine, engine::general_purpose::STANDARD};
      STANDARD.encode(input.as_bytes())
  }
  ```
- [ ] **Note**: the closure-wrapping in the legacy adapter is a TEMPORARY shim. The closures here exist only to bridge the old `Event::ClipboardLoad`/`ColorRequest` API for one migration phase. The handlers that EMIT no longer create closures — they emit `HostRequest` with a `ResponseToken`, and the legacy adapter wraps the token in a closure for old consumers. The wrapping closures are owned by the legacy adapter, not the VTE handler, and they go away when section X (some future section, after this plan completes) migrates the last legacy consumer.
- [ ] Sibling tests in `oriterm_core/src/effect/sink/legacy/tests.rs`:
  - `pty_write_routes_to_pty_write_event()`
  - `host_bell_routes_to_bell_event()`
  - `host_title_set_routes_to_title_event()`
  - `host_request_clipboard_load_routes_to_clipboard_load_event_with_reply_token()`
  - `host_request_color_query_routes_to_color_request_event_with_reply_token()`
  - `presentation_effects_dropped_silently()` (legacy listeners don't consume them)
- [ ] **Validation**: `cargo test -p oriterm_core --lib effect::sink::legacy::tests` passes.

---

## 03.5 Migrate VTE handler emission sites to emit Effect

**File(s):** `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/osc.rs`, `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/dcs.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/handler/image/kitty.rs`

The VTE handlers currently call `self.event_listener.send_event(Event::Foo(...))`. They need to change to call `self.effect_sink.push(Effect::Foo(...))`. The `Term<T>` struct gains an `effect_sink: Arc<dyn EffectSink>` field; existing consumers wrap their EventListener in a `LegacyEventSink::new(listener)` to keep working.

- [ ] Add `effect_sink: Arc<dyn EffectSink>` field to `Term<T>` in `oriterm_core/src/term/mod.rs`. Initialize from a constructor parameter.
- [ ] Update `Term::new()` signature to accept the sink. Add a `Term::new_with_legacy_listener()` convenience constructor that wraps an `EventListener` in a `LegacyEventSink` for back-compat with existing call sites (oriterm_mux's IO thread, tests).
- [ ] Migrate emission sites:
  - `handler/mod.rs:135` — `Event::Bell` → `effect_sink.push(Effect::Host(HostEffect::Bell))`
  - `handler/osc.rs:28` — `Event::Title(...)` → `effect_sink.push(Effect::Host(HostEffect::TitleSet { value: Some(t) }))`
  - `handler/osc.rs:34` — `Event::ResetTitle` → `effect_sink.push(Effect::Host(HostEffect::TitleSet { value: None }))`
  - `handler/osc.rs:44` — `Event::IconName(...)` → `effect_sink.push(Effect::Host(HostEffect::IconNameSet { value: Some(n) }))`
  - `handler/osc.rs:48` — `Event::ResetIconName` → similar
  - `handler/osc.rs:138` — `Event::ClipboardStore(...)` → `effect_sink.push(Effect::HostRequest(HostRequest::ClipboardStore { ... }))`
  - **`handler/osc.rs:153-159` — REMOVE the closure**, replace with `let token = ResponseToken::new(); effect_sink.push(Effect::HostRequest(HostRequest::ClipboardLoad { selection, reply: token.clone() })); /* token consumed by adapter */`
  - **`handler/osc.rs:102-110` — REMOVE the closure**, replace with `HostRequest::ColorQuery` similarly
  - `handler/modes.rs` cursor blink change emission → `Effect::Ui(UiEffect::CursorBlinkChanged { ... })`
  - `handler/modes.rs` mouse cursor dirty emission → `Effect::Ui(UiEffect::MouseCursorDirty)`
  - `handler/status.rs` (CPR, DSR, DA replies) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::CursorReport / DeviceStatus / DeviceAttribute })`
  - `handler/dcs.rs` (DECRQSS reply) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::StatusString })`
  - `handler/image/kitty.rs:465` (kitty image protocol ACK/error) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::ImageProtocolReply })`
- [ ] Update existing call sites that construct `Term::new()` to use `Term::new_with_legacy_listener()` so they don't break.
- [ ] **[BLOAT watch]** `oriterm_core/src/term/handler/mod.rs` is currently at 489 lines — near the 500-line limit. The handler migration in 03.5 adds `effect_sink` field access and modifies several emission sites; if the edits push the file past 500 lines, split `handler/mod.rs` into `mod.rs` (dispatch hub) + `handler/emit.rs` (the new emission helpers) BEFORE adding the new code. Do not leave `mod.rs` over-limit even briefly.
- [ ] **[BLOAT watch]** `oriterm_core/src/term/handler/image/kitty.rs` is currently at 476 lines. 03.5 migrates line 465 (kitty image protocol ACK/error) to emit `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply })`. If the migration pushes the file over 500 lines, split by extracting the reply-formatting helpers into `image/kitty/reply.rs`.
- [ ] **Validation**: existing tests in `oriterm_core/tests/teseq/`, `oriterm_core/tests/tack/`, and the alloc/RSS regression tests pass without modification. The legacy adapter preserves observable behavior.

---

## 03.6 Migrate Term::pending_notifications into the Effect channel

**File(s):** `oriterm_core/src/term/shell_state.rs`, `oriterm_core/src/raw_intercept/` (or wherever the raw interceptor lives that pushes notifications), `oriterm_core/src/term/mod.rs`

Per Codex Round 2, `Term::pending_notifications` is a split-brain side channel that the new Effect sink must absorb. Notifications are appended via `Term::push_notification()` from the raw interceptor (OSC 9/99/777) and drained via `Term::drain_notifications()`. After this subsection, notifications flow through `effect_sink.push(Effect::Host(HostEffect::DesktopNotification { ... }))` and consumers drain via `EffectSink::take_pending()`.

- [ ] Find every call site of `Term::push_notification()` in the raw interceptor (search for `push_notification`). Each one becomes:
  ```rust
  // OLD
  term.push_notification(Notification { title, body });

  // NEW
  term.effect_sink().push(Effect::Host(HostEffect::DesktopNotification {
      source: NotificationSource::Osc9, // or Osc99/Osc777 depending on which OSC fired
      title,
      body,
  }));
  ```
- [ ] In `oriterm_core/src/term/shell_state.rs`:
  - Remove the `pending_notifications: Vec<Notification>` field from `Term`.
  - Remove `Term::push_notification()` method.
  - Remove `Term::drain_notifications()` method (or keep as a thin shim that drains the effect sink and filters for `DesktopNotification` variants — for one-phase migration, prefer the shim).
- [ ] **[WASTE]** `oriterm_core/src/term/tests.rs:1598-1631` — `ris_clears_pending_notifications`, `drain_notifications_returns_empty_on_second_call`, and adjacent tests call `Term::push_notification` / `Term::drain_notifications` directly. After the field is removed, rewrite these tests to push via `effect_sink().push(Effect::Host(HostEffect::DesktopNotification { .. }))` and drain via `effect_sink().take_pending()`. The RIS semantic MUST still hold (RIS clears any host-pending notifications) — add an `EffectExpectation` assertion that the effect channel is empty after RIS.
- [ ] **[DRIFT]** `oriterm_core/src/term/mod.rs:172,240` — the `pending_notifications: Vec<Notification>` field declaration and its `Vec::new()` initialization are the parallel sync points that must be removed in lockstep with the shell_state.rs removal. Leaving the field alive after removing its setter/drainer creates dead memory.
- [ ] If any consumer was calling `term.drain_notifications()` (search for it), update them to call `effect_sink.take_pending()` and filter for the `DesktopNotification` variant.
- [ ] Add tests in `oriterm_core/src/term/shell_state/tests.rs` (sibling file):
  - `osc_9_pushes_desktop_notification_effect()`
  - `osc_99_pushes_desktop_notification_effect_with_osc99_source()`
  - `osc_777_pushes_desktop_notification_effect_with_osc777_source()`
- [ ] **Validation**: `grep -rn 'pending_notifications\|push_notification\|drain_notifications' oriterm_core/src/` returns no production matches (only tests if any).
- [ ] **TPR checkpoint** — `/tpr-review` covering 03.4–03.6 (legacy adapter + handler migration + notifications absorption). Catches integration issues from the multi-file migration.

---

## 03.7 Mark ClipboardLoad/ColorRequest closure variants deprecated; remove closures from emission sites

**File(s):** `oriterm_core/src/event/mod.rs`, sibling tests, and any remaining closure references

**Title clarification:** This subsection does NOT physically remove the `ClipboardLoad` and `ColorRequest` Event variants. Those stay in place as **deprecated** shims used only by the `LegacyEventSink` adapter (see 03.4). What this subsection removes is closure CONSTRUCTION at emission sites (handler files). After this subsection, handler files have zero `Arc<dyn Fn…>` / `Arc::new(move …)` occurrences; closures only exist inside the legacy-adapter shim.

After 03.5 emits via Effect/HostRequest, and 03.4's legacy adapter routes back through the existing Event for back-compat, the closure-carrying Event variants are technically still on the wire — but only because the legacy adapter creates them. The closure-formatter logic that USED to live in the OSC handler is gone (replaced by handler emission of `HostRequest`). At this point we can either keep the legacy adapter wrapping in closures (one-phase migration) OR remove the closure-bearing Event variants entirely and force consumers to subscribe to Effect directly.

Per Codex Round 2 ("production interface, not test-only ... migration via LegacyEventSink for one phase, then full migration"), this section adopts the **gradual approach**: closure variants stay in `Event` for now, the legacy adapter wraps closures around the response tokens, but the **handler emission sites no longer create closures**. The closures live only in the adapter, where they will be deleted when consumers migrate to subscribe to Effect directly — per the `plans/effect-cutover/` follow-up plan directory that section 03.N requires to be filed as an in-scope artifact. The frontmatter `success_criteria` and the mission criterion in `00-overview.md` are phrased consistently with this gradual approach — closures are REMOVED FROM EMISSION SITES, not from the enum itself.

- [ ] Verify zero closure construction in handler files:
  ```bash
  grep -rn 'Arc<dyn Fn\|Arc::new(move' oriterm_core/src/term/handler/
  ```
  must return zero matches in the handler files. Closures in the legacy adapter are OK at this stage.
- [ ] **[WASTE]** `oriterm_core/src/term/handler/osc.rs:142-159` — remove the stale doc comment "Sends a `ClipboardLoad` event with a closure that formats the base64-encoded response" once the closure construction is gone. The closure-with-formatter pattern is exactly what 03.5 eliminates. Replace with doc pointing at `HostRequest::ClipboardLoad` + `ResponseToken`.
- [ ] **[WASTE]** `oriterm_core/src/term/handler/tack_cap_xcheck/osc_clipboard.rs:7,38,42-43` — stale comments referencing `Event::ClipboardLoad` + "response-formatter closure". After section 03.5 migrates the emission site, update the test assertion to observe `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` directly (via the harness's effect transcript) and delete the legacy "ClipboardLoad" string matching. Currently matches against `events.iter().any(|e| e.contains("ClipboardLoad"))` — fragile string-match that should become a structured effect match.
- [ ] **[LEAK:scattered-knowledge]** `oriterm_core/src/term/handler/esc.rs:52` — RIS (reset to initial state) handler calls `self.pending_notifications.clear()` directly. After 03.6 removes `pending_notifications` from `Term`, this line must be replaced with `self.effect_sink().clear_host_notifications()` (or equivalent drain+discard through the Effect channel). Flag added to section 03 because the field removal in 03.6 will break the build here; the fix is part of the migration wave, not a separate pass.
- [ ] Add a deprecation comment on `Event::ClipboardLoad` and `Event::ColorRequest`:
  ```rust
  /// **Deprecated**: emitted only via `LegacyEventSink` adapter during the
  /// Effect migration. New code should subscribe to `Effect::HostRequest`
  /// directly. This variant will be removed when the last legacy consumer
  /// migrates (out of scope for spec-conformance plan).
  ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Send + Sync>),
  ```
- [ ] **Validation**: `grep -rn 'Arc<dyn Fn' oriterm_core/src/event/ oriterm_core/src/term/handler/osc.rs oriterm_core/src/term/handler/` returns matches ONLY in `oriterm_core/src/event/mod.rs` (the deprecated variants) and `oriterm_core/src/effect/sink/legacy.rs` (the adapter shim). No matches in the handler emission sites.

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 03.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD): tests in 03.1, 03.2, 03.3, 03.4, 03.6 written before implementation
- [ ] **Matrix dimensions**: Effect family × emission site × consumer routing — all 5 families × every relevant emission site (Bell, Title, IconName, Cwd, ClipboardStore, ClipboardLoad, ColorQuery, CursorBlink, MouseCursorDirty, PtyWrite for DA/CPR/DSR/DECRQSS/kitty-image-reply, DesktopNotification for OSC 9/99/777) × LegacyEventSink routing test
- [ ] **Semantic pin**: at least one test that PASSES only when closures are gone — `compile_fail` or grep-based assertion that `Arc<dyn Fn` does not appear in handler files
- [ ] All Effect type variants defined in `oriterm_core::effect`
- [ ] EffectSink trait + QueueingEffectSink + VoidEffectSink + LegacyEventSink all implemented
- [ ] snapshot_seqno field added and increments correctly under sync/non-sync conditions
- [ ] All VTE handler emission sites migrated to emit Effect (verified by grep — no `Arc::new(move` in handler files)
- [ ] `Term::pending_notifications` bypass channel removed; OSC 9/99/777 flow through Effect
- [ ] LegacyEventSink adapter routes Effect → existing Event for back-compat; existing tests pass without modification
- [ ] No new file exceeds 500 lines (split if needed; effect.rs, sink.rs, families/*.rs are all leaf files)
- [ ] No closures in handler files: `grep -rn 'Arc<dyn Fn\|Arc::new(move' oriterm_core/src/term/handler/` returns zero
- [ ] **Follow-up cutover plan exists**: `plans/effect-cutover/` directory is committed with `index.md`, `00-overview.md`, and at least one reviewed section file (e.g. `section-01-migrate-mux-consumer.md`) describing the migration of each current `Event::ClipboardLoad`/`ColorRequest` consumer to subscribe to `Effect::HostRequest` directly, plus a section that deletes the deprecated variants after the migration. This is the in-scope artifact that closes the "Deprecated closure Event variants scheduled for deletion" mission criterion — it is NOT a deferral dodge; the plan directory must exist before section 03 can be marked complete.
- [ ] Alloc regression unchanged: `cargo test -p oriterm_core --test alloc_regression` passes (closure removal must not introduce per-frame allocation)
- [ ] `./build-all.sh` green (cross-compile to x86_64-pc-windows-gnu also)
- [ ] `./test-all.sh` green debug + release
- [ ] `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 03 status updated
- [ ] `/tpr-review` passed (final, full-section) — independent Codex review
- [ ] `/impl-hygiene-review last commit` passed — hygiene review clean. MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** `oriterm_core::effect::Effect` exists as the production interface; closures removed from VTE handler emission; `Term::pending_notifications` bypass absorbed; LegacyEventSink bridges existing consumers; snapshot_seqno observable for section 04 + section 06; full test suite green debug + release; alloc regression unchanged.
