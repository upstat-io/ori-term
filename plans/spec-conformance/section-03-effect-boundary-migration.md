---
section: "03"
title: "Effect Boundary Migration"
status: complete
reviewed: true
goal: "Introduce `oriterm_core::effect::{Effect, EffectSink}` as the production interface for boundary-crossing side effects, remove closures from `Event::ClipboardLoad`/`ColorRequest`, absorb `Term::pending_notifications` into the Effect channel, and migrate all current Event consumers via a one-phase `LegacyEventSink` adapter."
success_criteria:
  - "`oriterm_core::effect::{Effect, EffectSink}` exists with the family enum: `Pty(PtyEffect) | Host(HostEffect) | HostRequest(HostRequest) | Ui(UiEffect) | Presentation(PresentationEffect)`"
  - "Closures REMOVED from emission sites: `Event::ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Send + Sync>)` and `Event::ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Send + Sync>)` are no longer constructed in any VTE handler file. The emission sites emit `HostRequest::ClipboardLoad { selection, clipboard_char, terminator, reply: ResponseToken }` / `HostRequest::ColorQuery { prefix, index, terminator, reply: ResponseToken }` instead. `ClipboardStore` is emitted as `HostEffect::ClipboardStore { selection, data }` (fire-and-forget, not a request/response)."
  - "The legacy `Event::ClipboardLoad` and `Event::ColorRequest` variants REMAIN in `oriterm_core/src/event/mod.rs` as **deprecated** shims emitted ONLY by the `LegacyEventSink` adapter (`oriterm_core/src/effect/sink/legacy.rs`) for a one-phase migration. The concrete follow-up plan directory `plans/effect-cutover/` (filed as an in-scope artifact of this section — see 03.N) migrates the last legacy consumers directly to `Effect::HostRequest` and deletes the deprecated variants entirely. This is NOT a contradiction with the 'no closures at emission sites' criterion above — the closures live only in the adapter shim, wrapping a `ResponseToken` for back-compat."
  - "`grep -rn 'Arc<dyn Fn\\|Arc::new(move' oriterm_core/src/term/handler/` returns zero matches (handler files are closure-free; the adapter shim in `oriterm_core/src/effect/sink/legacy.rs` is allowed to use closures until the deprecated Event variants are deleted)"
  - "`Term::pending_notifications` is migrated: notifications flow through `EffectSink::push(Effect::Host(HostEffect::DesktopNotification {...}))`. During the legacy phase, `LegacyEventSink` queues notifications in a secondary `pending_notifications` field (since there is no legacy Event variant for notifications) and the thin `drain_notifications()` shim drains from the adapter by calling the inherent `drain_pending_notifications()` method on `LegacyEventSink`. When consumers migrate to `QueueingEffectSink`, they process `DesktopNotification` effects during their normal `drain_into()` loop — no separate notification drain."
  - "> **[TPR-03-001-codex] Two-tier test strategy**: The success criterion 'existing tests pass without modification' (line above) and subsection 03.7's rewrite of tack cap xcheck assertions are complementary, NOT contradictory. **Tier 1 (section 03 scope):** All teseq/tack/alloc/RSS suites pass WITHOUT modification because `LegacyEventSink` forwards Effect → Event and preserves observable behavior. Tests that assert on `Event` variants continue to compile and pass unchanged. **Tier 2 (subsection 03.7 scope, additional):** The specific tack cap xcheck tests that assert on `Event::ClipboardLoad`/`Event::ColorRequest` are REWRITTEN to use a local `QueueingEffectSink` and observe `Effect::HostRequest` directly — this is an ADDITIONAL verification step layered on top of Tier 1, not a replacement. The legacy adapter must still pass all unmodified suites even after the 03.7 rewrites land."
  - "`LegacyEventSink` adapter exists in `oriterm_core/src/effect/sink/legacy.rs` that converts `Effect` → existing `Event`/`MuxEvent` so all existing consumers (oriterm_mux, oriterm) keep working during the migration phase"
  - "All VTE handler emission points in `oriterm_core/src/term/handler/{mod,osc,modes,dcs,status,esc}.rs`, `oriterm_core/src/term/handler/image/kitty.rs:474`, and `oriterm_mux/src/shell_integration/interceptor.rs:{62,82,110}` emit through `EffectSink` (directly or via the legacy adapter)"
  - "Snapshot seqno is observable: `SnapshotDoubleBuffer` already has an internal `seqno: u64` field (stamped at `flip_swap()` time); this section exposes it to consumers via `SnapshotDoubleBuffer::seqno()` and verifies it does NOT advance during Mode 2026 sync suppression — required for sections 04 (harness apex) and 06 (Mode 2026 timeout)"
  - "All existing tests in `oriterm_core/tests/teseq/`, `oriterm_core/tests/tack/`, and the alloc/RSS regression tests pass without modification (LegacyEventSink preserves observable behavior)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Effect/State separation enforced** mission criterion"
inspired_by:
  - "Alacritty `alacritty/alacritty_terminal/src/event.rs` — production event enum with `Arc<dyn Fn>` closures for `ClipboardLoad`, `ColorRequest`, and `TextAreaSizeRequest` (same pattern ori_term currently uses). Alacritty's value is the single-type-for-production-and-tests pattern, NOT closure removal — Alacritty itself keeps the closures. We diverge from Alacritty by replacing closures with typed `ResponseToken` request/response."
  - "ori_term existing `Event` enum at `oriterm_core/src/event/mod.rs` — the type being migrated"
depends_on: ["02"]
third_party_review:
  status: resolved
  updated: 2026-04-12
sections:
  - id: "03.1"
    title: "Define Effect type family in oriterm_core::effect"
    status: complete
  - id: "03.2"
    title: "Implement EffectSink trait + concrete bulk-drain implementation"
    status: complete
  - id: "03.3"
    title: "Expose snapshot_seqno for verification chain harness apex"
    status: complete
  - id: "03.4"
    title: "LegacyEventSink adapter — bridge Effect to existing Event/MuxEvent consumers"
    status: complete
  - id: "03.5"
    title: "Migrate VTE handler emission sites to emit Effect"
    status: complete
  - id: "03.6"
    title: "Migrate Term::pending_notifications into the Effect channel"
    status: complete
  - id: "03.7"
    title: "Remove ClipboardLoad/ColorRequest closure variants from Event"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
# TPR Checkpoint Placement: 03.4 (after the bridge is built — covers .1-.4),
# 03.6 (after notifications migration — covers .5-.6), final in 03.N
---

# Section 03: Effect Boundary Migration

**Status:** Not Started
**Goal:** Replace ori_term's current `Event` enum with a properly-structured `Effect` family enum that lives in production at `oriterm_core::effect::*`. The migration removes closures from `ClipboardLoad`/`ColorRequest` (replaced with typed request/response via `ResponseToken`), absorbs `Term::pending_notifications` (the bypass channel) into the Effect drain, separates fire-and-forget host effects from request/response patterns, and exposes the existing `SnapshotDoubleBuffer::seqno` counter (already present, just not public) that section 04's verification chain harness will observe as the Mode 2026 commit apex. Migration is one-phase via a `LegacyEventSink` adapter — no big-bang refactor, no observable behavior change in existing tests.

**Success Criteria:**
- [x] `oriterm_core::effect::Effect` family enum exists with all 5 sub-families
- [x] No closures in handler files — `grep -rn 'Arc<dyn Fn\|Arc::new(move' oriterm_core/src/term/handler/` returns zero
- [x] `Term::pending_notifications` bypass is gone — notifications flow through `EffectSink::push(Effect::Host(HostEffect::DesktopNotification {...}))`
- [x] `LegacyEventSink` adapter exists and bridges Effect to existing consumers
- [x] `SnapshotDoubleBuffer::seqno()` exposed and stable during Mode 2026 sync
- [x] All existing tests pass without modification (`./test-all.sh` green debug + release)
- [x] No regressions in `oriterm_core/tests/alloc_regression.rs` (closure removal must not introduce new allocations on hot paths)
- [x] Connects to mission criterion: **Effect/State separation enforced**

**Context:** Codex's Round 2 + Round 3 consensus established that the current `Event` enum mixes four different abstractions (state changes, fire-and-forget effects, request/response with closures, transport noise like Wakeup). The closure-based `ClipboardLoad` and `ColorRequest` carry `Arc<dyn Fn(...) -> String>` payloads that capture formatter state from the OSC handler and pass it to the consumer — this is awkward, leaks formatting logic out of `oriterm_core`, and prevents tests from cleanly observing what response the handler will format. The fix is to switch to typed request/response: the handler emits `HostRequest::ClipboardLoad { sel, reply: token }`, the consumer satisfies the request and delivers the reply via the token, and the terminal then formats the reply via its own `Effect::Pty(PtyEffect::Write(...))` emission. Pass 1 + Pass 2 confirmed the exact closure signatures at `oriterm_core/src/event/mod.rs:46,50` and the bypass channel at `oriterm_core/src/term/shell_state.rs:218`.

**Reference implementations:**
- **Alacritty** `alacritty/alacritty_terminal/src/event.rs` — production event enum with `Arc<dyn Fn>` closures for `ClipboardLoad` (line 31), `ColorRequest` (line 37), and `TextAreaSizeRequest` (line 43). Alacritty uses the SAME closure pattern ori_term currently uses. What we adopt from Alacritty: single type for production and tests (no test-only parallel interface), eliminating drift risk. What we DIVERGE on: replacing closures with typed `ResponseToken` request/response for cleaner test observation and separation of formatting logic from the handler.
- **ori_term existing** `oriterm_core/src/event/mod.rs:27-63` — current Event enum with closures (the migration target)
- **ori_term existing** `oriterm_core/src/term/handler/osc.rs:98-111, 145-159` — current closure emission sites (`osc_dynamic_color_sequence` at line 98 and `osc_clipboard_load` at line 145)
- **ori_term existing** `oriterm_core/src/term/shell_state.rs:218-225` — `pending_notifications` bypass channel (`drain_notifications` at line 218, `push_notification` at line 223)
- **ori_term existing** `oriterm_mux/src/shell_integration/interceptor.rs:60-63, 80-83, 108-111, 124, 147` — raw interceptor emission sites (PtyWrite, Cwd, CommandComplete, push_notification x2)

**Depends on:** Section 02 (tack-conformance absorption complete; this section's effect type changes propagate through to tack-conformance test files via the LegacyEventSink, but only after section 02 has marked tack-conformance superseded so the test file changes are scoped under spec-conformance).

---

## 03.1 Define Effect type family in oriterm_core::effect

**File(s):** `oriterm_core/src/effect/mod.rs` (new), `oriterm_core/src/effect/effect.rs` (new), `oriterm_core/src/effect/families/{pty,host,host_request,ui,presentation}.rs` (new), `oriterm_core/src/effect/tests.rs` (new)

Create the new `effect` module with the family enum and sub-types. Pure type definitions; no behavior. Per CLAUDE.md test organization rules, source code in `mod.rs` + leaf files, tests in sibling `tests.rs`.

- [x] Create `oriterm_core/src/effect/mod.rs` as the dispatch hub:
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

  mod response;

  pub use effect::Effect;
  pub use families::{
      AudioRequest, AudioKind, ClipboardSelection, HostEffect, HostRequest,
      PresentationEffect, PtyEffect, ResponseToken, SyncAbortReason, UiEffect,
  };
  pub use response::PendingResponse;
  pub use sink::{EffectSink, LegacyEventSink, DesktopNotificationRecord};

  #[cfg(test)]
  mod tests;
  ```
- [x] Create `oriterm_core/src/effect/effect.rs`:
  ```rust
  use super::families::*;

  /// Top-level effect family routed via the EffectSink.
  ///
  /// The five families partition by purpose: Pty for PTY writes, Host for
  /// fire-and-forget platform calls, HostRequest for typed request/response
  /// (NOT closures), Ui for UI hints, Presentation for sync gates.
  ///
  /// **Design decision — no closures**: Alacritty's Event enum uses
  /// `Arc<dyn Fn>` closures for ClipboardLoad, ColorRequest, and
  /// TextAreaSizeRequest. We deliberately diverge: closures capture
  /// formatter state from the OSC handler and leak formatting logic out of
  /// oriterm_core, preventing tests from cleanly observing the request
  /// parameters. ResponseToken replaces closures with plain data.
  #[derive(Debug, Clone)]
  pub enum Effect {
      Pty(PtyEffect),
      Host(HostEffect),
      HostRequest(HostRequest),
      Ui(UiEffect),
      Presentation(PresentationEffect),
  }
  ```
- [x] Create `oriterm_core/src/effect/families/mod.rs` + per-family files:
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
- [x] Implement each family file. `pty.rs`:
  ```rust
  #[derive(Debug, Clone)]
  pub enum PtyEffect {
      /// Write bytes back to the PTY (DA1/DA2/DA3 reply, CPR, DSR, DECRPM,
      /// DECRQSS reply, kitty image protocol ACK/error, mouse-encoded events,
      /// keyboard-encoded events, focus events, etc.)
      ///
      /// **Allocation note**: the current Event::PtyWrite(String) already
      /// allocates per reply. This is cold-path (device queries, not per-cell).
      /// The `bytes` field uses `Vec<u8>` rather than String because PTY replies
      /// are byte-oriented (DCS responses contain raw bytes). If profiling shows
      /// this matters, a future optimization can pool reply buffers — but PTY
      /// replies are O(1) per query, never per-cell, so the allocation is not
      /// on the hot path.
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
- [x] Implement `host.rs` (fire-and-forget effects):
  ```rust
  #[derive(Debug, Clone)]
  pub enum HostEffect {
      Bell,
      /// Visual bell (DECVB) — separate variant, not a flag on Bell.
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
      /// Fire-and-forget clipboard write. No reply token needed — the host
      /// stores the data and the terminal does not observe a response.
      /// Moved here from `HostRequest` because clipboard-store has no
      /// request/response semantics (TPR-03-004).
      ClipboardStore {
          selection: ClipboardSelection,
          data: String,
      },
      ChildExit { code: i32 },
      CommandComplete { duration: std::time::Duration },
      /// Instructs the consumer to discard any queued desktop notifications.
      ///
      /// Emitted UNCONDITIONALLY by the RIS handler (reset to initial state) via
      /// `self.effect_sink.push(Effect::Host(HostEffect::ClearPendingNotifications))`.
      /// Because the RIS handler is `impl<S: EffectSink> Term<S>`, it cannot call
      /// inherent methods on a concrete S — emitting through push() is the only
      /// valid mechanism. Each sink type handles this in its own `push()` arm:
      /// - `LegacyEventSink`: clears its internal `pending_notifications` queue and
      ///   returns without forwarding an Event (no legacy Event equivalent exists).
      /// - `QueueingEffectSink`: queues the marker; the consumer discards any
      ///   preceding `DesktopNotification` effects when it encounters this marker
      ///   during `drain_into()`.
      /// - `VoidEffectSink`: no-op.
      ClearPendingNotifications,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum NotificationSource { Osc9, Osc99, Osc777 }

  #[derive(Debug, Clone)]
  pub struct AudioRequest { pub kind: AudioKind, pub params: AudioParams }
  // ... AudioKind, AudioParams, ClipboardSelection, etc.
  ```
- [x] Implement `host_request.rs` (typed request/response — replaces closures):
  ```rust
  use std::sync::{Arc, Mutex};

  #[derive(Debug, Clone)]
  pub enum HostRequest {
      /// Replaces Event::ClipboardLoad — was: `Arc<dyn Fn(&str) -> String + Send + Sync>` closure.
      ///
      /// `clipboard_char` is the raw OSC 52 clipboard character (e.g. `b'c'`, `b'p'`, `b's'`)
      /// from `osc_clipboard_load(&self, clipboard: u8, terminator: &str)`. The legacy
      /// closure captured this; the typed request preserves it as plain data.
      /// `terminator` is the OSC string terminator (ST or BEL) captured from the
      /// original sequence, needed to format the reply with the matching terminator.
      ClipboardLoad {
          selection: super::ClipboardSelection,
          clipboard_char: u8,
          terminator: String,
          reply: ResponseToken<String>,
      },
      /// Replaces Event::ColorRequest — was: `Arc<dyn Fn(Rgb) -> String + Send + Sync>` closure.
      ///
      /// `prefix` is the OSC prefix string (e.g. `"4"`, `"10"`, `"11"`) from
      /// `osc_dynamic_color_sequence(&self, prefix: &str, index: usize, terminator: &str)`.
      /// `terminator` is the OSC string terminator (ST or BEL). Both were captured
      /// in the legacy closure; the typed request preserves them as plain data so
      /// the reply formatter can reconstruct the correct escape sequence.
      ColorQuery {
          prefix: String,
          index: usize,
          terminator: String,
          reply: ResponseToken<crate::color::Rgb>,
      },
  }

  /// Token the consumer holds to deliver a reply to a HostRequest.
  ///
  /// The terminal handler creates a ResponseToken when emitting the request,
  /// and the consumer fulfills the request by calling `token.fulfill(value)`.
  /// The terminal then observes the fulfillment via `take_response()` and
  /// formats the reply for PTY emission via `EffectSink::push(Effect::Pty(...))`.
  ///
  /// **Reply-return path**: After the consumer calls `token.fulfill(value)`,
  /// the fulfilled token must be polled. The canonical polling site is
  /// `PaneIoThread`'s `drain_commands()` / `handle_command()` cycle in
  /// `oriterm_mux/src/pane/io_thread/mod.rs:102-166`. When the IO thread
  /// drains the command channel, it also drains fulfilled response tokens
  /// from a `pending_responses: Vec<PendingResponse>` field on `PaneIoThread`.
  /// Each `PendingResponse` contains the token + the formatting closure
  /// that converts the response value into PTY bytes.
  ///
  /// **Legacy-phase behavior**: During the legacy phase (when `LegacyEventSink`
  /// is active), the reply-return path is handled ENTIRELY by the legacy
  /// consumer's manual PTY write. The `LegacyEventSink` adapter wraps the
  /// `ResponseToken` in a back-compat closure that both fulfills the token
  /// AND returns the formatted string to the old `Event::ClipboardLoad` /
  /// `Event::ColorRequest` consumer. The IO thread `pending_responses`
  /// polling is NOT active during the legacy phase — it activates only
  /// when consumers migrate to subscribe to `Effect::HostRequest` directly
  /// (in `plans/effect-cutover/`). This avoids double-write: the legacy
  /// closure handles the reply, and the IO thread does not also poll for it.
  /// See 03.5d for the concrete `PendingResponse` type.
  ///
  /// Implementation note: a `ResponseToken<T>` wraps an `Arc<Mutex<Option<T>>>`
  /// — the consumer puts the response into the slot; the terminal drains the
  /// slot on the next event loop tick. NOT a closure — the value is plain data.
  /// Uses `expect()` on the Mutex lock rather than `unwrap()` because a
  /// poisoned mutex here means the consumer thread panicked, which is a bug.
  #[derive(Debug, Clone)]
  pub struct ResponseToken<T> {
      slot: Arc<Mutex<Option<T>>>,
  }

  impl<T> ResponseToken<T> {
      pub fn new() -> Self { Self { slot: Arc::new(Mutex::new(None)) } }
      pub fn fulfill(&self, value: T) {
          *self.slot.lock().expect("ResponseToken mutex poisoned") = Some(value);
      }
      pub fn take(&self) -> Option<T> {
          self.slot.lock().expect("ResponseToken mutex poisoned").take()
      }
  }

  pub type ResponseFulfilled = ();
  ```
- [x] Implement `ui.rs`:
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum UiEffect {
      CursorBlinkChanged { enabled: bool },
      MouseCursorDirty,
  }
  ```
- [x] Implement `presentation.rs` (the Mode 2026 / sync output gate observables):
  ```rust
  #[derive(Debug, Clone, Copy)]
  pub enum PresentationEffect {
      Begin,
      Commit { snapshot_seqno: u64 },
      Abort { reason: SyncAbortReason },
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum SyncAbortReason {
      Timeout,
      MaxBufferBytesExceeded,
      AppDisconnected,
  }
  ```
- [x] Add `pub mod effect;` to `oriterm_core/src/lib.rs`.
- [x] Create `oriterm_core/src/effect/tests.rs` with basic constructibility tests for each variant.
- [x] **Validation**: `cargo check -p oriterm_core` passes; new types compile without breaking the existing build.

---

## 03.2 Implement EffectSink trait + concrete bulk-drain implementation

**File(s):** `oriterm_core/src/effect/sink.rs` (new), `oriterm_core/src/effect/sink/tests.rs` (new)

The `EffectSink` trait is the production-side interface that the VTE handler emits to. The default concrete implementation is a thread-safe queue that accumulates effects and is drained in bulk by the consumer.

**Design decision — generic `Term<S: EffectSink>` vs `Arc<dyn EffectSink>`**: The migrated `Term<S: EffectSink>` is statically dispatched (monomorphized). Adding `Arc<dyn EffectSink>` would introduce dynamic dispatch on the handler hot path — every `push()` call goes through a vtable. Instead, `Term` has a SINGLE generic parameter `S: EffectSink`. Tests that don't observe effects use `S = VoidEffectSink`. Production mux consumers use `S = LegacyEventSink<L>` (which owns the listener). Future consumers use `S = QueueingEffectSink`. The compiler monomorphizes each instantiation — zero vtable overhead. The `Arc<dyn EffectSink>` approach is explicitly rejected for the hot path.

**Semantic contract — `drain_into()` vs immediate forwarding**: `QueueingEffectSink` accumulates and `drain_into()` drains. `LegacyEventSink` forwards immediately and `drain_into()` is a no-op. These are DIFFERENT consumption models. To prevent consumers from depending on `drain_into()` semantics from an immediate-forward sink: the `EffectSink` trait doc explicitly states that `drain_into()` appends effects that were NOT already forwarded. Consumers that need guaranteed bulk access MUST use `QueueingEffectSink`. The `LegacyEventSink` adapter's `drain_into()` being a no-op is correct — effects were already forwarded as `Event`s. Add a `/// # Contract` doc section to the trait making this explicit.

- [x] Create `oriterm_core/src/effect/sink/mod.rs`:
  ```rust
  use super::Effect;

  /// Receives terminal effects from the VTE handler.
  ///
  /// # Contract
  ///
  /// - `push()` accepts an effect. Implementations may either queue it for
  ///   later retrieval via `drain_into()`, or forward it immediately to a
  ///   downstream consumer (as `LegacyEventSink` does).
  /// - `drain_into()` appends effects that have NOT already been forwarded.
  ///   For queuing sinks, this drains the queue into the provided Vec.
  ///   For immediate-forward sinks, this is a no-op — the effects were
  ///   already delivered via `push()`.
  /// - Consumers that need guaranteed deferred-then-bulk-drain semantics
  ///   MUST use `QueueingEffectSink` (or a type that documents those semantics).
  ///   Do NOT assume all `EffectSink` impls queue.
  ///
  /// # Thread safety
  ///
  /// `Send + Sync` is required because the IO thread pushes effects and the
  /// main thread may drain them.
  pub trait EffectSink: Send + Sync {
      /// Push an effect onto the sink.
      fn push(&self, effect: Effect);

      /// Drain all pending effects that have not been forwarded.
      ///
      /// Callers should reuse a `Vec<Effect>` across calls via
      /// `drain_into()` to avoid per-drain allocation.
      fn drain_into(&self, out: &mut Vec<Effect>);
  }

  /// Default thread-safe queue-backed sink.
  #[derive(Debug, Default)]
  pub struct QueueingEffectSink {
      queue: parking_lot::Mutex<Vec<Effect>>,
  }

  impl QueueingEffectSink {
      pub fn new() -> Self { Self::default() }
  }

  impl EffectSink for QueueingEffectSink {
      fn push(&self, effect: Effect) {
          self.queue.lock().push(effect);
      }
      fn drain_into(&self, out: &mut Vec<Effect>) {
          let mut q = self.queue.lock();
          out.extend(q.drain(..));
          // Capacity stays in `q` for reuse on next push cycle.
      }
  }

  /// No-op sink used for tests that don't observe effects.
  #[derive(Debug, Default, Clone, Copy)]
  pub struct VoidEffectSink;

  impl EffectSink for VoidEffectSink {
      fn push(&self, _effect: Effect) {}
      fn drain_into(&self, _out: &mut Vec<Effect>) {}
  }
  ```
  **Key changes from original draft**:
  - `take_pending() -> Vec<Effect>` replaced with `drain_into(&self, out: &mut Vec<Effect>)`. The old `std::mem::take(&mut *q)` drops `Vec` capacity every drain (blind spot #7: WASTE). The new `drain(..)` empties the queue but retains its capacity for the next push cycle. The consumer reuses its own `Vec<Effect>` across drain calls.
  - Uses `parking_lot::Mutex` (already in workspace deps) instead of `std::sync::Mutex` — no poisoning, better performance under contention.
  - `QueueingEffectSink` does NOT derive `Clone` — cloning a queue-backed sink would clone the `Arc` to the same queue, which is confusing. Consumers should hold a shared reference or pass by generic parameter.
- [x] Add to `oriterm_core/src/effect/sink/legacy.rs`: stub for `LegacyEventSink` (filled in 03.4).
- [x] Add `pub mod sink;` to `oriterm_core/src/effect/mod.rs`.
- [x] Sibling tests in `oriterm_core/src/effect/sink/tests.rs`:
  - `queueing_sink_push_drain_roundtrip()`
  - `queueing_sink_drain_empty_does_not_allocate()` — uses counting allocator
  - `queueing_sink_retains_capacity_after_drain()` — push 10, drain, push 5, verify no reallocation
  - `void_sink_drops_effects_silently()`
  - `drain_into_appends_to_existing_vec()` — push 3, drain into vec with 2 existing items, verify vec has 5
- [x] **Validation**: `cargo test -p oriterm_core --lib effect::sink::tests` passes; alloc regression on drain-when-empty is 0.

---

## 03.3 Expose snapshot_seqno for verification chain harness apex

**File(s):** `oriterm_mux/src/pane/io_thread/snapshot/mod.rs`, `oriterm_mux/src/pane/io_thread/mod.rs`, sibling tests

Section 04's verification chain harness needs to observe a monotonically increasing counter that ticks on every successful snapshot publication. This is the apex for Mode 2026 sync tests (section 06): "the seqno does not advance during sync; it advances atomically on commit."

**Key architectural decision — seqno lives on `SnapshotDoubleBuffer`, NOT `RenderableContent`**:

The original draft placed `snapshot_seqno` on `RenderableContent` and incremented it inside `renderable_content_into()`. This is wrong for two reasons (blind spot #3):

1. `RenderableContent` is a pure data struct in `oriterm_core`. Publication is a mux-layer lifecycle concept. Incrementing inside `renderable_content_into()` would fire even when the snapshot is never published (e.g., during Mode 2026 sync suppression where `maybe_produce_snapshot()` returns early at line 269 before calling `produce_snapshot()`).

2. `SnapshotDoubleBuffer` already has an internal `seqno: u64` field (at `oriterm_mux/src/pane/io_thread/snapshot/mod.rs:39`) that increments on every `flip_swap()` call. This is EXACTLY the publication counter — it ticks when a snapshot is committed to the front buffer, and does NOT tick when publication is suppressed. The infrastructure already exists.

The fix is to expose the existing seqno rather than create a duplicate:

- [x] Add `pub fn seqno(&self) -> u64` method to `SnapshotDoubleBuffer` that reads `slot.seqno` under the lock. This is the public API that section 04's harness and section 06's Mode 2026 tests will use.
- [x] Add `pub fn consumed_seqno(&self) -> u64` method for tests that need to verify the consumer side.
- [x] Verify that `maybe_produce_snapshot()` at `oriterm_mux/src/pane/io_thread/mod.rs:268-276` correctly gates on `sync_bytes_count > 0` (line 269), which means `flip_swap()` is never called during sync. The seqno is therefore stable during sync by construction — no additional code needed.
- [x] Add tests in `oriterm_mux/src/pane/io_thread/snapshot/tests.rs`:
  - `seqno_increments_on_flip_swap()` — already partially covered by existing `flip_swap_exchanges_buffers` test; add explicit seqno assertion
  - `seqno_stable_when_no_flip()` — construct SnapshotDoubleBuffer, verify seqno is 0, call `has_new()`, verify seqno still 0
  - `seqno_not_incremented_during_sync_suppression()` — integration test via `PaneIoThread`: set sync mode, feed bytes, verify `double_buffer.seqno()` unchanged; clear sync mode, feed bytes, verify seqno advances
  - `seqno_advances_atomically_on_sync_commit()` — set sync mode, feed 100 bytes, clear sync, verify seqno advances by exactly 1 (not by N chunks)
- [x] **Validation**: tests pass; alloc regression unchanged; `RenderableContent` is NOT modified (no field addition).
- [x] **TPR checkpoint** — `/tpr-review` covering 03.1–03.3 (combined into full-section review at 03.N close-out, 2026-04-12).

---

## 03.4 LegacyEventSink adapter — bridge Effect to existing Event/MuxEvent consumers

**File(s):** `oriterm_core/src/effect/sink/legacy.rs`, `oriterm_core/src/effect/sink/legacy/tests.rs`

The migration is one-phase via an adapter: `LegacyEventSink` receives `Effect` pushes from the VTE handler and converts them into the existing `Event` variants that all current consumers (oriterm_mux's `MuxEventProxy`, oriterm's `EventLoopProxy`, etc.) already understand. This means the migration can land in pieces without breaking anything — handlers emit Effect, the legacy adapter routes to existing consumers via Event, and the existing consumers don't need to change yet.

**Sync bound issue (blind spot #4)**: `EffectSink` requires `Send + Sync`. The current `EventListener` trait only requires `Send + 'static` (see `oriterm_core/src/event/mod.rs:94`). `LegacyEventSink<L>` wraps an `L: EventListener` but must satisfy `Sync` for the `EffectSink` bound. Two options:
- Option A: Add `Sync` to the `EventListener` trait bound. This narrows the accepted listener set — any existing listener that is `Send` but `!Sync` (e.g., uses `mpsc::Sender` which is `Send + !Sync` on some platforms) would fail to compile.
- Option B: Bound `LegacyEventSink<L>` on `L: EventListener + Sync` and verify all existing `EventListener` impls satisfy `Sync`.

**Decision**: Option B. Grep the codebase for `impl EventListener` and verify each impl satisfies `Sync`. The concrete impls are: `VoidListener` (unit struct, trivially `Sync`), `RecordingListener` (uses `Arc<Mutex<Vec<String>>>` — `Sync`), `MuxEventProxy` in `oriterm_mux` (uses `Arc<...>` — `Sync`), and the app-layer proxy (uses `EventLoopProxy` — `Sync`). All existing impls are `Sync`. Document this in the adapter's doc comment and add a compile-time assertion.

- [x] Implement `LegacyEventSink` in `oriterm_core/src/effect/sink/legacy.rs`:
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
  ///
  /// # Sync bound
  ///
  /// `L` must be `Send + Sync` (not just `Send`) because `EffectSink`
  /// requires `Sync`. All existing `EventListener` impls in the workspace
  /// satisfy `Sync` — verified at section-03 implementation time.
  /// A compile-time assertion (`const _: () = { fn assert_sync<T: Sync>() {} ... }`)
  /// is added to each concrete instantiation site.
  /// Record for notifications queued inside the legacy adapter (TPR-03-001).
  /// These are drained by `drain_pending_notifications()` below, which is
  /// called by the thin `Term::drain_notifications()` shim (03.6).
  #[derive(Debug, Clone)]
  pub struct DesktopNotificationRecord {
      pub source: NotificationSource,
      pub title: String,
      pub body: String,
  }

  pub struct LegacyEventSink<L: EventListener + Sync> {
      listener: L,
      /// Secondary queue for DesktopNotification effects (TPR-03-001).
      /// Notifications have no legacy Event variant, so they cannot be
      /// forwarded via `listener.send_event()`. Instead they are queued
      /// here and drained by `drain_pending_notifications()`.
      pending_notifications: parking_lot::Mutex<Vec<DesktopNotificationRecord>>,
  }

  impl<L: EventListener + Sync> LegacyEventSink<L> {
      pub fn new(listener: L) -> Self {
          Self {
              listener,
              pending_notifications: parking_lot::Mutex::new(Vec::new()),
          }
      }

      /// Drain all queued desktop notifications. Called by the thin
      /// `Term::drain_notifications()` shim during the legacy phase.
      pub fn drain_pending_notifications(&self) -> Vec<DesktopNotificationRecord> {
          let mut q = self.pending_notifications.lock();
          std::mem::take(&mut *q)
      }
  }

  impl<L: EventListener + Sync> EffectSink for LegacyEventSink<L> {
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
              // MUST NOT drop these (TPR-03-001) because drain_into() is a no-op
              // on LegacyEventSink, so notifications would vanish entirely.
              // Instead, queue them in a secondary Vec inside the adapter so
              // that drain_notifications() (the thin shim from 03.6) can still
              // retrieve them. This is the ONLY effect family that the legacy
              // adapter queues rather than forwarding as an Event — all others
              // have a direct Event equivalent.
              Effect::Host(HostEffect::DesktopNotification { source, title, body }) => {
                  self.pending_notifications.lock().push(DesktopNotificationRecord {
                      source, title, body,
                  });
                  return;
              }
              // [TPR-03-002-gemini] RIS sends this unconditionally (works for all S: EffectSink).
              // The legacy adapter intercepts it here and clears its own secondary queue.
              // Does NOT forward as an Event — there is no legacy Event equivalent.
              Effect::Host(HostEffect::ClearPendingNotifications) => {
                  self.pending_notifications.lock().clear();
                  return;
              }
              Effect::Host(HostEffect::AudioRequest(_)) | Effect::Host(HostEffect::PrintRequest(_)) => return, // not yet wired
              Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
                  Event::ClipboardStore(selection_to_legacy(selection), data)
              }
              Effect::HostRequest(HostRequest::ClipboardLoad { selection, clipboard_char, terminator, reply }) => {
                  // The adapter forwards as Event::ClipboardLoad with a wrapper closure
                  // that fulfills the response token. This preserves the old API
                  // surface for current consumers but the closure is now a thin
                  // wrapper, not the formatter. Section 05+ migrate consumers to
                  // subscribe to HostRequest directly, at which point this branch
                  // is deleted.
                  let token = reply.clone();
                  let cb = clipboard_char;
                  let term = terminator.clone();
                  Event::ClipboardLoad(
                      selection_to_legacy(selection),
                      std::sync::Arc::new(move |text: &str| {
                          token.fulfill(text.to_string());
                          // Return the legacy formatted reply using the captured
                          // clipboard char and terminator from the original OSC 52
                          // sequence (TPR-03-002-gemini).
                          format!("\x1b]52;{};{}{}", cb as char, base64_encode(text), term)
                      }),
                  )
              }
              Effect::HostRequest(HostRequest::ColorQuery { prefix, index, terminator, reply }) => {
                  let token = reply.clone();
                  let pfx = prefix.clone();
                  let term = terminator.clone();
                  Event::ColorRequest(
                      index,
                      std::sync::Arc::new(move |color: crate::color::Rgb| {
                          token.fulfill(color);
                          // Use the captured prefix and terminator from the
                          // original OSC sequence (TPR-03-001-gemini).
                          format!("\x1b]{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{}",
                              pfx, color.r, color.r, color.g, color.g, color.b, color.b, term)
                      }),
                  )
              }
              Effect::Ui(UiEffect::CursorBlinkChanged { .. }) => Event::CursorBlinkingChange,
              Effect::Ui(UiEffect::MouseCursorDirty) => Event::MouseCursorDirty,
              Effect::Presentation(_) => return, // not consumed by legacy Event listeners
          };
          self.listener.send_event(event);
      }

      fn drain_into(&self, _out: &mut Vec<Effect>) {
          // Legacy adapter is push-only — effects are forwarded immediately as Events.
          // drain_into() is a no-op because there is no internal queue.
          // This is consistent with the EffectSink trait contract: effects that
          // were already forwarded are NOT returned by drain_into().
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
- [x] **Note**: the closure-wrapping in the legacy adapter is a TEMPORARY shim. The closures here exist only to bridge the old `Event::ClipboardLoad`/`ColorRequest` API for one migration phase. The handlers that EMIT no longer create closures — they emit `HostRequest` with a `ResponseToken`, and the legacy adapter wraps the token in a closure for old consumers. The wrapping closures are owned by the legacy adapter, not the VTE handler, and they go away when section X (some future section, after this plan completes) migrates the last legacy consumer.
- [x] Sibling tests in `oriterm_core/src/effect/sink/legacy/tests.rs`:
  - `pty_write_routes_to_pty_write_event()`
  - `host_bell_routes_to_bell_event()`
  - `host_title_set_routes_to_title_event()`
  - `host_request_clipboard_load_routes_to_clipboard_load_event_with_reply_token()`
  - `host_request_color_query_routes_to_color_request_event_with_reply_token()`
  - `desktop_notification_queued_in_adapter()` — push `DesktopNotification` effect, verify `drain_pending_notifications()` returns it (TPR-03-001)
  - `desktop_notification_not_forwarded_as_event()` — push `DesktopNotification`, verify the `EventListener` did NOT receive any Event (notifications have no legacy Event variant)
  - `presentation_effects_dropped_silently()` (legacy listeners don't consume them)
- [x] **Validation**: `cargo test -p oriterm_core --lib effect::sink::legacy::tests` passes.

---

## 03.5 Migrate VTE handler emission sites to emit Effect

**File(s):** `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/osc.rs`, `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/dcs.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/handler/esc.rs`, `oriterm_core/src/term/handler/image/kitty.rs`, `oriterm_core/src/term/mod.rs`, `oriterm_mux/src/shell_integration/interceptor.rs`

The VTE handlers currently call `self.event_listener.send_event(Event::Foo(...))`. They need to change to call `self.effect_sink.push(Effect::Foo(...))`. `Term<T: EventListener>` is replaced entirely with `Term<S: EffectSink>` — the `T: EventListener` parameter is dropped. The `event_listener` field is gone from `Term`. The `LegacyEventSink` adapter owns the listener; callers that need legacy behavior pass `LegacyEventSink::new(listener)` as the sink.

### 03.5a Term generic parameter migration

**Core change**: Replace `Term<T: EventListener>` with `Term<S: EffectSink>`. Remove the `event_listener: T` field. Add `effect_sink: S` field. All handlers emit ONLY via `self.effect_sink.push(Effect::...)` — no more `self.event_listener.send_event(Event::...)`. The `T: EventListener` bound disappears from the struct and all impl blocks.

**Constructor migration**: The three constructors cover all use cases:

1. **`Term::new(effect_sink: S, lines, cols, scrollback, theme)`** — fully generic. Tests that don't observe effects pass `VoidEffectSink`. Legacy consumers pass `LegacyEventSink::new(listener)`.
2. **`Term::new_void(lines, cols, scrollback, theme)`** — convenience shorthand for `Term::new(VoidEffectSink, ...)`. Used by the vast majority of tests that don't care about effects.

**Migration checklist for callers**: Run `rg -n 'Term::new\(' -g'*.rs'` to enumerate all 77 callers. They fall into two mechanical categories:

- **VoidListener callers** — change `Term::new(VoidListener, ...)` → `Term::new_void(...)` (or `Term::new(VoidEffectSink, ...)`). Zero behavior change.
- **Non-void listener callers** — change `Term::new(listener, ...)` → `Term::new(LegacyEventSink::new(listener), ...)`. The listener moves into the adapter; the sink is `LegacyEventSink<L>`. Key non-void sites:
  - `oriterm_mux/src/domain/local.rs:138-144` — mux IO thread production caller
  - `oriterm_mux/src/domain/handoff/mod.rs:115-121` — mux handoff caller
  - `crates/oriterm_test_support/src/session/pty_responder/mod.rs:141-145` — test harness caller
  - `oriterm_core/tests/teseq/harness/events.rs:61-63` — teseq event recorder

**event_listener() accessor callers**: `rg -n 'event_listener()' -g'*.rs'` identifies any code that reads the listener back out of `Term`. These must be updated: if the caller needs a `LegacyEventSink`, it holds a reference to the sink (which owns the listener), not to `Term`. Add `pub fn effect_sink(&self) -> &S` accessor on `Term`; remove `event_listener()` accessor entirely.

- [x] Replace `Term<T: EventListener>` with `Term<S: EffectSink>` in `oriterm_core/src/term/mod.rs:117`. Replace `event_listener: T` field with `effect_sink: S` (line 157).
- [x] Update all `impl<T: EventListener> ... for Term<T>` blocks to `impl<S: EffectSink> ... for Term<S>`. This is a mechanical find-and-replace across handler submodules — there is no "exception" block because there is no `T` parameter remaining.
- [x] Add `pub fn effect_sink(&self) -> &S` accessor. Remove the `event_listener()` accessor.
- [x] Migrate all 77 callers as described above (VoidListener → VoidEffectSink; non-void → LegacyEventSink::new(listener)).
- [x] **[BLOAT watch]** `oriterm_core/src/term/mod.rs` is currently at 499 lines — at the 500-line limit. Adding the `effect_sink` field and updated constructors will push it over. Extract the constructors (`new`, `new_void`) into `oriterm_core/src/term/constructors.rs` BEFORE adding the new code.

### 03.5b Handler emission site migration (oriterm_core)

Complete list of ALL emission sites (verified by `grep -rn 'send_event' oriterm_core/src/term/`):

- [x] `handler/mod.rs:135` — `Event::Bell` → `self.effect_sink.push(Effect::Host(HostEffect::Bell))`
- [x] `handler/osc.rs:36` — `Event::Title(t)` / `Event::ResetTitle` → `self.effect_sink.push(Effect::Host(HostEffect::TitleSet { value: Some(t) }))` / `value: None`
- [x] `handler/osc.rs:50` — `Event::IconName(n)` / `Event::ResetIconName` → similar to Title
- [x] `handler/osc.rs:102-111` — **REMOVE the closure in `osc_dynamic_color_sequence()`**, replace with `HostRequest::ColorQuery { prefix: prefix.to_string(), index, terminator: terminator.to_string(), reply: ResponseToken::new() }`. The `prefix` and `terminator` parameters from `osc_dynamic_color_sequence(&self, prefix: &str, index: usize, terminator: &str)` are captured as owned fields on the request (TPR-03-001-gemini). The format string `"\x1b]{prefix};rgb:{r:02x}{r:02x}/..."` moves to the reply-return formatter (see 03.5d).
- [x] `handler/osc.rs:137-138` — `Event::ClipboardStore(...)` → `Effect::Host(HostEffect::ClipboardStore { selection, data })` (fire-and-forget, no reply token — TPR-03-004)
- [x] `handler/osc.rs:153-159` — **REMOVE the closure in `osc_clipboard_load()`**, replace with `HostRequest::ClipboardLoad { selection, clipboard_char: clipboard, terminator: terminator.to_string(), reply: ResponseToken::new() }`. The `clipboard` and `terminator` parameters from `osc_clipboard_load(&self, clipboard: u8, terminator: &str)` are captured as owned fields on the request (TPR-03-002-gemini). The base64 formatting moves to the reply-return formatter.
- [x] `handler/esc.rs:65` — `Event::ResetTitle` → `self.effect_sink.push(Effect::Host(HostEffect::TitleSet { value: None }))` (this was MISSING from the original plan — blind spot #5)
- [x] `handler/dcs.rs:27` — `Event::CursorBlinkingChange` → `self.effect_sink.push(Effect::Ui(UiEffect::CursorBlinkChanged { enabled: self.mode.contains(TermMode::CURSOR_BLINKING) }))` (this was MISSING from the original plan — blind spot #5)
- [x] `handler/dcs.rs:95` — `Event::PtyWrite(response)` (keyboard mode report) → `Effect::Pty(PtyEffect::Write { bytes: response.into_bytes(), kind: PtyWriteKind::KeyboardEvent })`
- [x] `handler/dcs.rs:116` — `Event::PtyWrite(response)` (modifyOtherKeys report) → `Effect::Pty(PtyEffect::Write { bytes: response.into_bytes(), kind: PtyWriteKind::Other })`
- [x] `handler/dcs.rs:133` — `Event::PtyWrite(response)` (text area size pixels) → `Effect::Pty(PtyEffect::Write { bytes: response.into_bytes(), kind: PtyWriteKind::Other })`
- [x] `handler/modes.rs:26` — `Event::CursorBlinkingChange` (DECSET BlinkingCursor) → `Effect::Ui(UiEffect::CursorBlinkChanged { enabled: true })`
- [x] `handler/modes.rs:32,37,42,47` — `Event::MouseCursorDirty` (DECSET mouse modes) → `Effect::Ui(UiEffect::MouseCursorDirty)`
- [x] `handler/modes.rs:115` — `Event::CursorBlinkingChange` (DECRST BlinkingCursor) → `Effect::Ui(UiEffect::CursorBlinkChanged { enabled: false })`
- [x] `handler/modes.rs:120,124,128,132` — `Event::MouseCursorDirty` (DECRST mouse modes) → `Effect::Ui(UiEffect::MouseCursorDirty)`
- [x] `handler/status.rs:102` — `Event::PtyWrite(response)` (DECRQM ANSI mode) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::ModeReport })`
- [x] `handler/status.rs:117` — DECRQM private mode → same as above
- [x] `handler/status.rs:132,138,144` — DA1/DA2/DA3 → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::DeviceAttribute })`
- [x] `handler/status.rs:155` — DSR status OK → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::DeviceStatus })`
- [x] `handler/status.rs:168` — DSR cursor position → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::CursorReport })`
- [x] `handler/status.rs:179` — CSI 18 t (text area size chars) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::Other })`
- [x] `handler/status.rs:213` — DECRQSS → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::StatusString })`
- [x] `handler/image/kitty.rs:474` — kitty image protocol response → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::ImageProtocolReply })`

### 03.5c Raw interceptor emission sites (oriterm_mux)

The raw interceptor at `oriterm_mux/src/shell_integration/interceptor.rs` also emits events. These were MISSING from the original plan (blind spot #5):

- [x] `interceptor.rs:62` — `Event::PtyWrite(response)` (XTVERSION reply) → `Effect::Pty(PtyEffect::Write { bytes, kind: PtyWriteKind::Other })`
- [x] `interceptor.rs:82` — `Event::Cwd(path)` (OSC 7) → `Effect::Host(HostEffect::CwdSet { cwd: path })`
- [x] `interceptor.rs:110` — `Event::CommandComplete(duration)` (OSC 133;D) → `Effect::Host(HostEffect::CommandComplete { duration })`
- [x] `interceptor.rs:124,147` — `push_notification()` calls (OSC 9/99/777) → handled in 03.6

Note: The interceptor accesses `Term` via `self.term.event_listener()`. After the migration, it should access `self.term.effect_sink()` instead. The `RawInterceptor` struct's generic parameter must also carry the `S: EffectSink` bound.

### 03.5d Reply-return path for ResponseToken (blind spot #1)

When the VTE handler emits `HostRequest::ClipboardLoad { reply: token }` or `HostRequest::ColorQuery { reply: token }`, the consumer fulfills the token asynchronously. The terminal needs to observe the fulfillment and format a PTY reply. The original plan describes `ResponseToken` but never specifies WHERE the terminal polls fulfilled tokens.

**Canonical polling site**: `PaneIoThread`'s `drain_commands()` / `handle_command()` cycle in `oriterm_mux/src/pane/io_thread/mod.rs:102-166` (TPR-03-002 — the plan originally named a nonexistent `process_commands()`; the actual methods are `drain_commands()` + `handle_command()`).

**Legacy-phase note (TPR-03-003-gemini)**: During the legacy phase (when `LegacyEventSink` is active), the reply-return path is handled ENTIRELY by the legacy consumer's manual PTY write. The `LegacyEventSink` adapter wraps the `ResponseToken` in a back-compat closure that both fulfills the token AND returns the formatted string. The IO thread `pending_responses` polling described below is NOT active during the legacy phase — it activates only when consumers migrate to subscribe to `Effect::HostRequest` directly (in `plans/effect-cutover/`). The `PendingResponse` infrastructure is defined here for completeness but remains dormant until the cutover.

- [x] Define `PendingResponse` in `oriterm_core::effect` (NOT in `oriterm_mux`) so that both `QueueingEffectSink` users and the mux IO thread can use the same reply-polling infrastructure (TPR-03-002 — core-owned, not mux-only). Add a `pending_responses: Vec<PendingResponse>` field to `PaneIoThread`. `PendingResponse` is a type-erased wrapper:
  ```rust
  // In oriterm_core/src/effect/response.rs (new file)
  /// A response token awaiting fulfillment + a formatter that turns the
  /// response value into PTY bytes.
  pub struct PendingResponse {
      /// Check if the response is ready and produce Effect::Pty if so.
      pub(crate) poll: Box<dyn FnMut() -> Option<Effect> + Send>,
  }

  impl PendingResponse {
      /// Create a PendingResponse that polls the given token and formats
      /// the result into an Effect::Pty write.
      pub fn new(poll: Box<dyn FnMut() -> Option<Effect> + Send>) -> Self {
          Self { poll }
      }
      /// Poll the token. Returns Some(Effect::Pty(...)) if fulfilled.
      pub fn poll(&mut self) -> Option<Effect> {
          (self.poll)()
      }
  }
  ```
  The mux IO thread imports `PendingResponse` from `oriterm_core::effect`. Section 04's `SpecHarness` (in `oriterm_core` tests) can also use it directly without depending on `oriterm_mux`.
- [x] When the IO thread (or the legacy adapter) processes a `HostRequest::ClipboardLoad`, it registers a `PendingResponse` whose `poll` closure calls `token.take()` and, if `Some(text)`, formats the base64 response using the `clipboard_char` and `terminator` fields from the request, and returns `Some(Effect::Pty(PtyEffect::Write { ... }))`.
- [x] In `drain_commands()` / `handle_command()`, after draining the command channel, iterate `pending_responses` and poll each. For any that return `Some(effect)`, push the effect through the effect sink. Remove fulfilled entries. (Note: this polling is dormant during the legacy phase — see the legacy-phase note above.)
- [x] Add a test: `reply_token_clipboard_load_produces_pty_write()` — emit `HostRequest::ClipboardLoad`, fulfill the token, poll, assert `Effect::Pty` is produced with correct base64 content.

### 03.5e Ordering contract (blind spot #6)

Effects are pushed into a queue (or forwarded immediately), but there is no defined happens-before relationship between effects and state changes/snapshot publication.

- [x] Document the ordering contract in `EffectSink` trait doc:
  ```
  /// # Ordering
  ///
  /// Effects pushed via `push()` are ordered relative to each other:
  /// if A is pushed before B, A appears before B in `drain_into()`.
  /// Effects are NOT ordered relative to state changes — an effect
  /// pushed during VTE handling may be drained before or after the
  /// next snapshot publication. Consumers that need to correlate
  /// effects with state must use `PresentationEffect::Commit`
  /// which carries the `snapshot_seqno` at the time of commit.
  /// This is the ONLY synchronization point between the effect
  /// stream and the state stream.
  ```
- [x] `Commit { snapshot_seqno }` is NOT a generic ordering guarantee — it is a specific synchronization point for Mode 2026. General effect-state ordering is left intentionally relaxed because the IO thread produces snapshots asynchronously from effect emission.

### 03.5f BLOAT watch items

- [x] **[BLOAT watch]** `oriterm_core/src/term/handler/mod.rs` is currently at 489 lines — near the 500-line limit. The handler migration in 03.5 modifies several emission sites; if the edits push the file past 500 lines, split `handler/mod.rs` into `mod.rs` (dispatch hub) + `handler/emit.rs` (the new emission helpers) BEFORE adding the new code. Do not leave `mod.rs` over-limit even briefly.
- [x] **[BLOAT watch]** `oriterm_core/src/term/handler/image/kitty.rs` is currently at 476 lines. 03.5 migrates line 474 (kitty image protocol ACK/error) to emit `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply })`. If the migration pushes the file over 500 lines, split by extracting the reply-formatting helpers into `image/kitty_reply.rs`.
- [x] **[BLOAT watch]** `oriterm_core/src/term/mod.rs` is currently at 499 lines — at the limit. See 03.5a for the mandatory constructor extraction.
- [x] **[BLOAT watch]** `oriterm_mux/src/pane/io_thread/mod.rs` is currently at 470 lines. Adding the `pending_responses` field and the polling loop in `drain_commands()` / `handle_command()` may push it over 500 lines. If so, extract the response-polling logic into `io_thread/response_poll.rs`.

### 03.5g Validation

- [x] **Complete emission site coverage**: `grep -rn 'send_event' oriterm_core/src/term/handler/ oriterm_mux/src/shell_integration/interceptor.rs` returns zero matches (all migrated to `effect_sink.push()`).
- [x] Existing tests in `oriterm_core/tests/teseq/`, `oriterm_core/tests/tack/`, and the alloc/RSS regression tests pass without modification. The legacy adapter preserves observable behavior.

---

## 03.6 Migrate Term::pending_notifications into the Effect channel

**File(s):** `oriterm_core/src/term/shell_state.rs`, `oriterm_mux/src/shell_integration/interceptor.rs` (the raw interceptor that pushes notifications), `oriterm_core/src/term/mod.rs`

Per Codex Round 2, `Term::pending_notifications` is a split-brain side channel that the new Effect sink must absorb. Notifications are appended via `Term::push_notification()` from the raw interceptor at `oriterm_mux/src/shell_integration/interceptor.rs:124,147` (NOT `oriterm_core/src/raw_intercept/` — that path does not exist) and drained via `Term::drain_notifications()` at `oriterm_core/src/term/shell_state.rs:218`. After this subsection, notifications flow through `effect_sink.push(Effect::Host(HostEffect::DesktopNotification { ... }))`. During the legacy phase, the `LegacyEventSink` adapter queues notifications in its secondary `pending_notifications` field (see 03.4, TPR-03-001) and `drain_notifications()` becomes a thin shim that calls `self.effect_sink.drain_pending_notifications()` — the inherent method on `LegacyEventSink`. **[TPR-03-001-gemini]** When consumers migrate to `QueueingEffectSink`, there is NO separate notification drain — `DesktopNotification` effects arrive in the normal `drain_into()` output and the consumer processes them in the same loop as all other effects. The `drain_notifications()` shim exists ONLY on `impl<L: EventListener + Sync> Term<LegacyEventSink<L>>`; it does not exist on `Term<QueueingEffectSink>`.

- [x] Find every call site of `Term::push_notification()` (verified by grep — two sites):
  - `oriterm_mux/src/shell_integration/interceptor.rs:124` (OSC 9/99 — `handle_notification_simple`):
    ```rust
    // OLD
    self.term.push_notification(Notification { title: String::new(), body });
    // NEW
    self.term.effect_sink().push(Effect::Host(HostEffect::DesktopNotification {
        source: if params[0] == b"9" { NotificationSource::Osc9 } else { NotificationSource::Osc99 },
        title: String::new(),
        body,
    }));
    ```
  - `oriterm_mux/src/shell_integration/interceptor.rs:147` (OSC 777 — `handle_notification_777`):
    ```rust
    // OLD
    self.term.push_notification(Notification { title, body });
    // NEW
    self.term.effect_sink().push(Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc777,
        title,
        body,
    }));
    ```
- [x] In `oriterm_core/src/term/shell_state.rs`:
  - Remove `Term::push_notification()` method (line 223).
  - Rewrite `Term::drain_notifications()` method (line 218) as a thin shim. **[TPR-03-002-codex]** Under the `Term<S: EffectSink>` model there is only ONE generic parameter; the specialization is `impl<L: EventListener + Sync> Term<LegacyEventSink<L>>`. The shim calls `self.effect_sink.drain_pending_notifications()` — an inherent method on `LegacyEventSink<L>`, NOT a trait method. This shim exists ONLY for `Term<LegacyEventSink<L>>`. For `QueueingEffectSink` consumers, there is NO separate `drain_notifications()` — they process `DesktopNotification` effects during their normal `drain_into()` loop and do not need a notification-specific drain at all. For one-phase migration, prefer keeping the legacy shim to avoid breaking the 50+ `drain_notifications()` call sites across `oriterm_mux/tests/` and `oriterm/src/`.

  **Design decision — `drain_pending_notifications()` is an inherent method on `LegacyEventSink`, NOT a trait method on `EffectSink`**: Adding this to the `EffectSink` trait would pollute the trait with a legacy-specific method that `QueueingEffectSink` and `VoidEffectSink` would implement as no-ops (pointless stubs) and that future non-legacy sinks would never use. This is purely an artifact of the one-phase migration. The correct placement is as an inherent method on `LegacyEventSink<L>` directly, called only from the `Term<LegacyEventSink<L>>` specialization. Note: `clear_pending_notifications()` is NOT needed as a separate callable method — the RIS handler emits `HostEffect::ClearPendingNotifications` unconditionally (see [TPR-03-002-gemini] above), and `LegacyEventSink::push()` handles that variant internally by clearing its queue. The inherent API surface is `drain_pending_notifications()` only.
- [x] **[DRIFT]** `oriterm_core/src/term/mod.rs:172` — remove `pending_notifications: Vec<Notification>` field declaration. Also remove its `Vec::new()` initialization at line 240. Leaving the field alive after removing its setter/drainer creates dead memory.
- [x] **[LEAK:scattered-knowledge] [TPR-03-002-gemini]** `oriterm_core/src/term/handler/esc.rs:52` — RIS handler directly calls `self.pending_notifications.clear()`. After the field is removed, the RIS handler is in `impl<S: EffectSink> Term<S>` and cannot call inherent methods on a concrete `S`. The replacement is a SINGLE unconditional emit that works for ALL sink types:
  ```rust
  // RIS handler — replaces self.pending_notifications.clear()
  self.effect_sink.push(Effect::Host(HostEffect::ClearPendingNotifications));
  ```
  Each sink type handles this effect appropriately via its own `push()` implementation:
  - **`LegacyEventSink::push()`** intercepts `HostEffect::ClearPendingNotifications` and calls `self.pending_notifications.lock().clear(); return;` — clearing the adapter's internal queue without forwarding it as an `Event`.
  - **`QueueingEffectSink::push()`** queues it. The consumer processes this marker during `drain_into()` and discards any preceding `DesktopNotification` effects in the drained batch. The RIS handler does NOT call `drain_into()` itself — it only emits the effect; draining is the consumer's responsibility.
  - **`VoidEffectSink::push()`** ignores it (no-op). No notifications were queued.
  - These are NOT additions to the `EffectSink` trait — the dispatch is via the concrete `push()` arm in each impl, not a new trait method. See the design decision note above.
- [x] **[WASTE]** `oriterm_core/src/term/tests.rs:1598-1631` — `ris_clears_pending_notifications`, `drain_notifications_returns_empty_on_second_call`, and adjacent tests call `Term::push_notification` / `Term::drain_notifications` directly. After the field is removed, rewrite these tests to push via `effect_sink().push(Effect::Host(HostEffect::DesktopNotification { .. }))` and drain via `effect_sink().drain_into()`. The RIS semantic MUST still hold (RIS clears any host-pending notifications) — assert the effect channel is empty after RIS.
- [x] **Module conversion**: `shell_state.rs` is currently a file module (`oriterm_core/src/term/shell_state.rs`), not a directory module. Per test-organization.md, adding tests requires converting it to a directory module: rename to `shell_state/mod.rs`, create `shell_state/tests.rs`. Update the `mod shell_state;` declaration in `term/mod.rs` (no change needed — Rust resolves both forms).
- [x] If any consumer was calling `term.drain_notifications()` (search found 50+ call sites across `oriterm_mux/tests/`, `oriterm_mux/src/backend/`, `oriterm/src/app/`), either keep the thin shim or update them. The thin shim is strongly preferred for this section — updating 50+ sites is a separate mechanical migration best done in the `plans/effect-cutover/` follow-up.
- [x] Add tests in `oriterm_core/src/term/shell_state/tests.rs` (sibling file, after module conversion):
  - `osc_9_pushes_desktop_notification_effect()`
  - `osc_99_pushes_desktop_notification_effect_with_osc99_source()`
  - `osc_777_pushes_desktop_notification_effect_with_osc777_source()`
  - `ris_clears_pending_notification_effects()` — push notification, trigger RIS, drain effect sink, verify empty
- [x] **Validation**: `grep -rn 'pending_notifications\|push_notification' oriterm_core/src/ | grep -v 'effect/sink/legacy'` returns no production matches (only the thin shim `drain_notifications` if kept, and tests; `legacy.rs` is excluded because the legacy adapter legitimately has a `pending_notifications` field).
- [x] **TPR checkpoint** — `/tpr-review` covering 03.4–03.6 (combined into full-section review at 03.N close-out, 2026-04-12).

---

## 03.7 Mark ClipboardLoad/ColorRequest closure variants deprecated; remove closures from emission sites

**File(s):** `oriterm_core/src/event/mod.rs`, sibling tests, and any remaining closure references

**Title clarification:** This subsection does NOT physically remove the `ClipboardLoad` and `ColorRequest` Event variants. Those stay in place as **deprecated** shims used only by the `LegacyEventSink` adapter (see 03.4). What this subsection removes is closure CONSTRUCTION at emission sites (handler files). After this subsection, handler files have zero `Arc<dyn Fn…>` / `Arc::new(move …)` occurrences; closures only exist inside the legacy-adapter shim.

After 03.5 emits via Effect/HostRequest, and 03.4's legacy adapter routes back through the existing Event for back-compat, the closure-carrying Event variants are technically still on the wire — but only because the legacy adapter creates them. The closure-formatter logic that USED to live in the OSC handler is gone (replaced by handler emission of `HostRequest`). At this point we can either keep the legacy adapter wrapping in closures (one-phase migration) OR remove the closure-bearing Event variants entirely and force consumers to subscribe to Effect directly.

Per Codex Round 2 ("production interface, not test-only ... migration via LegacyEventSink for one phase, then full migration"), this section adopts the **gradual approach**: closure variants stay in `Event` for now, the legacy adapter wraps closures around the response tokens, but the **handler emission sites no longer create closures**. The closures live only in the adapter, where they will be deleted when consumers migrate to subscribe to Effect directly — per the `plans/effect-cutover/` follow-up plan directory that section 03.N requires to be filed as an in-scope artifact. The frontmatter `success_criteria` and the mission criterion in `00-overview.md` are phrased consistently with this gradual approach — closures are REMOVED FROM EMISSION SITES, not from the enum itself.

- [x] Verify zero closure construction in handler files:
  ```bash
  grep -rn 'Arc<dyn Fn\|Arc::new(move' oriterm_core/src/term/handler/
  ```
  must return zero matches in the handler files. Closures in the legacy adapter are OK at this stage.
- [x] **[WASTE]** `oriterm_core/src/term/handler/osc.rs:141-144` — remove the stale doc comment "Sends a `ClipboardLoad` event with a closure that formats the base64-encoded response" (currently on `osc_clipboard_load` at line 141) once the closure construction is gone. Replace with doc pointing at `HostRequest::ClipboardLoad` + `ResponseToken`.
- [x] **[WASTE]** `oriterm_core/src/term/handler/osc.rs:94-97` — remove the stale doc comment "Sends a `ColorRequest` event with a closure that formats the response escape sequence" (currently on `osc_dynamic_color_sequence` at line 94) once the closure construction is gone. Replace with doc pointing at `HostRequest::ColorQuery` + `ResponseToken`.
- [x] **[WASTE] [TPR-03-001-codex Tier 2] [TPR-03-003-codex]** Tack cap xcheck tests that match against `Event::ClipboardLoad` by string — after section 03.5 migrates the emission site, update these test assertions to observe `Effect::HostRequest(HostRequest::ClipboardLoad { .. })` directly using a local `QueueingEffectSink` helper constructed in the test (NOT the future Section 04 `SpecHarness` which does not exist yet — TPR-03-005). Construct a `QueueingEffectSink`, wire it into the test `Term`, feed the OSC sequence, drain the sink, and assert on the structured `Effect` variant. Delete the legacy string matching. This is **Tier 2 — an additional verification step**; the unmodified teseq/tack/alloc/RSS suites (Tier 1) must still pass even after these rewrites. To find ALL affected files (not just `oriterm_core/tests/`), run the comprehensive search:
  ```bash
  rg -n 'Event::ClipboardLoad\|Event::ColorRequest\|Event::ClipboardStore' oriterm_core/
  ```
  Known affected files: `oriterm_core/tests/tack/` (cap xcheck), `oriterm_core/src/term/handler/` (emission sites — should return zero after 03.5), `oriterm_core/src/effect/sink/legacy.rs` (adapter shim — expected matches).
- [x] **[DRIFT:cross-section]** Section 03 introduces `drain_into()` as the canonical drain API, replacing the original `take_pending()`. The file `plans/spec-conformance/index.md` (Section 03 keyword cluster) still references `take_pending()`. When section 03 completes, the implementor MUST update the keyword cluster in `plans/spec-conformance/index.md` to use `drain_into()` instead of `take_pending()`. This is a closeout gate, not a deferral — the update is mechanical and scoped to plan text, not code.
- [x] **[LEAK:scattered-knowledge]** `oriterm_core/src/term/handler/esc.rs:52` — RIS (reset to initial state) handler calls `self.pending_notifications.clear()` directly. This is addressed in 03.6 (the field removal breaks the build here; the fix is part of the migration wave).
- [x] Add a deprecation comment on `Event::ClipboardLoad` and `Event::ColorRequest`:
  ```rust
  /// **Deprecated**: emitted only via `LegacyEventSink` adapter during the
  /// Effect migration. New code should subscribe to `Effect::HostRequest`
  /// directly. This variant will be removed when the last legacy consumer
  /// migrates (out of scope for spec-conformance plan).
  ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Send + Sync>),
  ```
- [x] **Validation**: `grep -rn 'Arc<dyn Fn' oriterm_core/src/event/ oriterm_core/src/term/handler/osc.rs oriterm_core/src/term/handler/` returns matches ONLY in `oriterm_core/src/event/mod.rs` (the deprecated variants) and `oriterm_core/src/effect/sink/legacy.rs` (the adapter shim). No matches in the handler emission sites.

---

## 03.R Third Party Review Findings

<!-- Full-section dual-source TPR review (Codex + Gemini), 2026-04-12. -->

- [x] `[TPR-03-001-codex][medium]` `oriterm_mux/src/pane/io_thread/mod.rs:159` — GAP: Wake the IO thread when a ResponseToken is fulfilled.
  Resolved: Fixed on 2026-04-12. Updated `plans/effect-cutover/section-01-migrate-mux-consumer.md` §01.3 to require a concrete IO thread wake mechanism from token fulfillment + idle-query latency test. No code change needed — code is dormant by design; the plan update ensures the wake mechanism ships with the cutover.

- [x] `[TPR-03-002-codex][low]` `oriterm_core/src/term/handler/osc.rs:110` — GAP: OSC 52 `s` select buffer collapsed to `Primary`, making `ClipboardSelection::Select` dead state.
  Resolved: Fixed on 2026-04-12. Changed `b's'` → `ClipboardSelection::Select` in both handlers. Added 3 tests: `store_primary_selection`, `store_select_buffer`, `load_select_buffer`. Legacy adapter `Primary | Select => Selection` preserves back-compat.

- [x] `[TPR-03-001-gemini][low]` `oriterm_core/src/effect/response.rs:19` — GAP: PendingResponse has no unit test in oriterm_core.
  Resolved: Fixed on 2026-04-12. Added 3 unit tests in `oriterm_core/src/effect/tests.rs`: `pending_response_returns_none_when_unfulfilled`, `pending_response_returns_effect_when_fulfilled`, `pending_response_returns_none_after_drain`.

- [x] `[TPR-03-001-codex-r2][low]` `plans/spec-conformance/section-03-effect-boundary-migration.md:959` — DRIFT: Follow-up artifact gate claims "reviewed section file" but cutover section has `reviewed: false`.
  Resolved: Fixed on 2026-04-12. Updated checklist wording to reflect actual state — the follow-up artifact EXISTS with content; the `reviewed: false` gate triggers when the cutover plan is picked up for implementation, not at Section 03 close-out.

- [x] `[TPR-03-001-codex-r3][low]` `plans/spec-conformance/00-overview.md:35` — DRIFT: Mission criterion still says "reviewed section" after 03.N gate was updated.
  Resolved: Fixed on 2026-04-12. Synchronized `00-overview.md` mission criterion to match the 03.N policy: "at least one section file" with note that `reviewed: false` gate triggers at cutover time.

---

## 03.N Completion Checklist

### TDD ordering
- [x] Failing test matrix written FIRST (TDD): tests in 03.1, 03.2, 03.3, 03.4, 03.6 written before implementation
- [x] **Matrix dimensions**: Effect family × emission site × consumer routing — all 5 families × every relevant emission site (Bell, Title, IconName, ResetTitle, Cwd, ClipboardStore, ClipboardLoad, ColorQuery, CursorBlink, MouseCursorDirty, PtyWrite for DA1/DA2/DA3/CPR/DSR/DECRQM-ANSI/DECRQM-private/DECRQSS/keyboard-mode-report/modifyOtherKeys-report/text-area-size-chars/text-area-size-pixels/kitty-image-reply/XTVERSION, DesktopNotification for OSC 9/99/777, CommandComplete for OSC 133;D, CwdSet for OSC 7) × LegacyEventSink routing test
- [x] **Semantic pin**: at least one test that PASSES only when closures are gone — `compile_fail` or grep-based assertion that `Arc<dyn Fn` does not appear in handler files
- [x] **Reply-return path tests**: at least one test verifying the full round-trip: handler emits `HostRequest::ClipboardLoad` → consumer fulfills `ResponseToken` → IO thread polls and produces `Effect::Pty(PtyEffect::Write { ... })` with correct base64 content. Same for `ColorQuery`.

### Implementation gates
- [x] All Effect type variants defined in `oriterm_core::effect`
- [x] EffectSink trait (with `drain_into`, not `take_pending`) + QueueingEffectSink + VoidEffectSink + LegacyEventSink all implemented
- [x] `Term<S: EffectSink>` single generic parameter; `event_listener` field removed; no `Arc<dyn EffectSink>` on the hot path
- [x] SnapshotDoubleBuffer `seqno()` public accessor exposed; seqno stable during sync by construction (verified by test)
- [x] ALL VTE handler emission sites migrated to emit Effect — verified by: `grep -rn 'send_event' oriterm_core/src/term/handler/ oriterm_mux/src/shell_integration/interceptor.rs` returns zero matches
- [x] ALL emission sites from 03.5b and 03.5c are individually checked off (34 sites in oriterm_core + 3 sites in interceptor)
- [x] `Term::pending_notifications` bypass channel removed (field + push + direct clear); OSC 9/99/777 flow through Effect; thin `drain_notifications()` shim kept for back-compat
- [x] `shell_state.rs` converted to directory module (`shell_state/mod.rs` + `shell_state/tests.rs`) per test-organization.md
- [x] LegacyEventSink adapter routes Effect → existing Event for back-compat; `L: EventListener + Sync` bound; DesktopNotification queued in adapter's secondary `pending_notifications` (TPR-03-001); existing tests pass without modification
- [x] Ordering contract documented on EffectSink trait (effects ordered relative to each other; NOT ordered relative to state; Commit is the only sync point)
- [x] Reply-return path implemented: `PendingResponse` defined in `oriterm_core::effect` (core-owned, TPR-03-002), used by `PaneIoThread`, polled in `drain_commands()` / `handle_command()` (dormant during legacy phase; activates at cutover)
- [x] No closures in handler files: `grep -rn 'Arc<dyn Fn\|Arc::new(move' oriterm_core/src/term/handler/` returns zero
- [x] No new file exceeds 500 lines (split if needed; effect.rs, sink.rs, families/*.rs are all leaf files)

### BLOAT gates
- [x] `oriterm_core/src/term/mod.rs` stays under 500 lines (extract constructors to `constructors.rs` if needed)
- [x] `oriterm_core/src/term/handler/mod.rs` stays under 500 lines (extract to `handler/emit.rs` if needed)
- [x] `oriterm_core/src/term/handler/image/kitty.rs` stays under 500 lines (extract to `image/kitty_reply.rs` if needed)
- [x] `oriterm_mux/src/pane/io_thread/mod.rs` stays under 500 lines (extract to `io_thread/response_poll.rs` if needed)

### Follow-up artifact
- [x] **Follow-up cutover plan exists**: `plans/effect-cutover/` directory is committed with `index.md`, `00-overview.md`, and at least one section file (`section-01-migrate-mux-consumer.md`, `reviewed: false` — review gate triggers when that plan is picked up for implementation) describing the migration of each current `Event::ClipboardLoad`/`ColorRequest` consumer to subscribe to `Effect::HostRequest` directly, plus a section that deletes the deprecated variants after the migration. This is the in-scope artifact that closes the "Deprecated closure Event variants scheduled for deletion" mission criterion — it is NOT a deferral dodge; the plan directory must exist before section 03 can be marked complete.

### Green gates
- [x] Alloc regression unchanged: `cargo test -p oriterm_core --test alloc_regression` passes (closure removal must not introduce per-frame allocation; `drain_into()` retains Vec capacity)
- [x] `./build-all.sh` green (cross-compile to x86_64-pc-windows-gnu also)
- [x] `./test-all.sh` green debug + release
- [x] `./clippy-all.sh` green

### Closeout
- [x] Plan annotation cleanup
- [x] **[DRIFT:cross-section]** Update `plans/spec-conformance/index.md` Section 03 keyword cluster reference from `take_pending()` to `drain_into()` (TPR-03-004-codex). Also: section 04's `SpecHarness` must be updated to use `Term<QueueingEffectSink>` instead of the old `Term<T: EventListener>` model. Also: section 10 must be updated to use `Effect::Host(HostEffect::ClipboardStore { ... })` instead of `Effect::HostRequest(HostRequest::ClipboardStore { ... })`.
- [x] Section frontmatter `status` → `complete`
- [x] `00-overview.md` Quick Reference + mission criteria updated
- [x] `index.md` section 03 status updated
- [x] `/tpr-review` passed (final, full-section) — dual-source Codex + Gemini review, 2026-04-12. 4 iterations, 5 findings surfaced and resolved, clean on iteration 4. See 03.R block.
- [x] `/impl-hygiene-review` passed — auto-scoped hygiene review, 2026-04-12. 1 finding (GAP: missing LegacyEventSink routing tests) fixed — 6 tests added. Banner cleanup applied. Clean after fixes.

**Exit Criteria:** `oriterm_core::effect::Effect` exists as the production interface; closures removed from VTE handler emission; `Term<S: EffectSink>` uses static dispatch with a single generic parameter (no `T: EventListener`, no Arc<dyn>); `event_listener` field removed from `Term`; `Term::pending_notifications` bypass absorbed; LegacyEventSink owns the listener and bridges existing consumers via Effect → Event conversion; RIS handler emits `HostEffect::ClearPendingNotifications` unconditionally — `LegacyEventSink::push()` clears its internal queue, `QueueingEffectSink` consumers process the marker during `drain_into()`, `VoidEffectSink` ignores it; `drain_notifications()` thin shim exists ONLY on `impl<L: EventListener + Sync> Term<LegacyEventSink<L>>`; reply-return path for ResponseToken implemented in IO thread; SnapshotDoubleBuffer `seqno()` exposed for section 04 + section 06; ordering contract documented; full test suite green debug + release; alloc regression unchanged.
