---
section: "10"
title: "OSC Suite (full)"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/osc.md` and `catalog/shell-integration.md` beyond the baseline subset (covered by section 08) from `implemented-unverified` to `verified`, including OSC 8 hyperlinks, OSC 22/50 cursor icon/shape, OSC 9/99/777 desktop notifications, OSC 104/110/111/112 color reset, OSC 133 semantic prompt, OSC 633 VS Code shell integration, and OSC 1337 minimal subset (full iTerm2 in section 14)."
success_criteria:
  - "Every row in `catalog/osc.md` not covered by section 08 is `verified`"
  - "Every row in `catalog/shell-integration.md` is `verified`"
  - "OSC 8 hyperlink rows verified — hyperlink survives reflow, scroll, copy, alt-screen toggle (cell metadata test, not visual)"
  - "OSC 52 clipboard rows verified — both `c` (copy) and `s` (selection) targets, both store and load directions; `HostRequest::ClipboardLoad` apex with `ResponseToken` reply tested"
  - "OSC 9 / 99 / 777 desktop notification rows verified — `Effect::Host(HostEffect::DesktopNotification { source: Osc9 / Osc99 / Osc777, ... })` apex tested for each variant"
  - "OSC 133 semantic prompt rows verified — OSC 133;A/B/C/D each emit the corresponding state change AND the prompt marker is correctly recorded in the term state"
  - "OSC 633 VS Code shell integration rows verified — VS Code-specific OSC variants emit the expected effects"
  - "OSC 1337 minimal subset (SetMark, RemoteHost, CurrentDir, ShellIntegrationVersion) verified; full iTerm2 inline images deferred to section 14"
  - "All existing teseq OSC tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "gist:egmontkob OSC 8 hyperlink spec — canonical hyperlink format"
  - "Final Term proposal — OSC 133 semantic prompt (FTCS_*)"
  - "iTerm2 docs — OSC 9, OSC 1337, OSC 22 (cursor)"
  - "VS Code source — OSC 633 shell integration variants"
  - "kitty docs — OSC 777 desktop notifications"
depends_on: ["03", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "10.1"
    title: "OSC 8 hyperlinks (cell metadata + lifecycle)"
    status: not-started
  - id: "10.2"
    title: "OSC 52 clipboard (store + load with ResponseToken)"
    status: not-started
  - id: "10.3"
    title: "OSC 9/99/777 desktop notifications"
    status: not-started
  - id: "10.4"
    title: "OSC 133 semantic prompt + OSC 633 VS Code shell integration"
    status: not-started
  - id: "10.5"
    title: "OSC 22/50 cursor icon/shape, OSC 104/110/111/112 color reset"
    status: not-started
  - id: "10.6"
    title: "OSC 1337 minimal subset (non-image variants)"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "10.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 10.3 (after notifications + clipboard — covers .1-.3),
# 10.6 (after shell integration + final OSC variants — covers .4-.6), final in 10.N
---

# Section 10: OSC Suite (full)

**Status:** Not Started
**Goal:** Verify every OSC catalog row beyond the baseline subset. The OSC suite is broad but mostly mechanical — each OSC number gets a spec_chain test that emits the OSC, observes the apex (state mutation, host effect, or PTY reply), and asserts.

**Success Criteria:** see frontmatter.

**Context:** Section 08 verifies the basic OSC subset (0/1/2 title, 4 palette, 7 CWD, 10/11/12 default colors, 52 clipboard basic). This section drives the rest. Notable: OSC 52 clipboard load is the first non-trivial use of `HostRequest` + `ResponseToken` from section 03 — the consumer (test or real consumer) fulfills the request and the terminal observes the fulfilled value. OSC 8 hyperlinks attach metadata to cells, so the apex test asserts cell-attached URI survives reflow/scroll/copy.

**Reference implementations:** see frontmatter.

**Depends on:** Section 08 (baseline OSC subset verified, basic OSC parsing solid).

---

## 10.1 OSC 8 hyperlinks

**File(s):** `oriterm_core/tests/spec_chain/osc/hyperlinks.rs` (new)

- [ ] Spec_chain test: emit `OSC 8 ; ; uri ST` followed by text followed by `OSC 8 ; ; ST` to terminate. Verify the cells in between have the URI attached (state apex via `RenderableCell::hyperlink_uri`).
- [ ] Test hyperlink with id parameter: `OSC 8 ; id=foo ; uri ST`
- [ ] Test hyperlink survives reflow: place hyperlink, resize the grid, verify cells still carry the URI
- [ ] Test hyperlink survives scroll: place hyperlink, scroll the viewport, verify the cells in scrollback still carry the URI
- [ ] Test hyperlink terminator behavior: subsequent text without OSC 8 ; ; ST does NOT carry the URI
- [ ] Update `catalog/osc.md` row OSC-8 to `verified`.
- [ ] **Validation**: tests pass; cell-attached URI confirmed across all transformations.

---

## 10.2 OSC 52 clipboard

**File(s):** `oriterm_core/tests/spec_chain/osc/clipboard.rs` (new)

OSC 52 has three forms: store (`OSC 52 ; c ; <base64> ST`), load (`OSC 52 ; c ; ? ST`), and reset. Store emits `Effect::HostRequest(HostRequest::ClipboardStore { selection, data })`; load emits `Effect::HostRequest(HostRequest::ClipboardLoad { selection, reply: token })` and the consumer fulfills the token, after which the terminal formats and emits the response via `Effect::Pty(PtyEffect::Write)`.

- [ ] Spec_chain test for OSC 52 store: emit `OSC 52 ; c ; SGVsbG8= ST`, assert `Effect::HostRequest(HostRequest::ClipboardStore { selection: Clipboard, data: "Hello" })` observed in the transcript.
- [ ] Spec_chain test for OSC 52 store with selection (`s` not `c`): same shape with `selection: Selection`.
- [ ] Spec_chain test for OSC 52 load: emit `OSC 52 ; c ; ? ST`, observe the `HostRequest::ClipboardLoad` request, fulfill the token in the test (`token.fulfill("test data".into())`), then assert the next observed effect is `Effect::Pty(PtyEffect::Write { bytes: ..., kind: ... })` with the formatted reply containing the base64-encoded "test data".
- [ ] Update `catalog/osc.md` rows OSC-52-STORE / OSC-52-LOAD to `verified`.
- [ ] **Validation**: store and load tests pass; ResponseToken round-trip confirmed.

---

## 10.3 OSC 9/99/777 desktop notifications

**File(s):** `oriterm_core/tests/spec_chain/osc/notifications.rs` (new)

- [ ] Spec_chain test for OSC 9: emit `OSC 9 ; <body> ST`, assert `Effect::Host(HostEffect::DesktopNotification { source: NotificationSource::Osc9, body, title: "" })` observed.
- [ ] Spec_chain test for OSC 99 (iTerm2 variant with title + body): emit `OSC 99 ; title ; body ST` (verify exact format from iTerm2 docs), assert `Effect::Host(HostEffect::DesktopNotification { source: Osc99, ... })`.
- [ ] Spec_chain test for OSC 777 (kitty variant): emit `OSC 777 ; notify ; title ; body ST`, assert `Effect::Host(HostEffect::DesktopNotification { source: Osc777, ... })`.
- [ ] Update catalog rows in `catalog/osc.md` to `verified`.
- [ ] **Validation**: all three notification variants emit the correct Effect with the correct source discriminator.
- [ ] **TPR checkpoint** — `/tpr-review` covering 10.1–10.3 (hyperlinks + clipboard + notifications). Catches HostRequest/ResponseToken integration issues before subsections 10.4-10.6 build on top.

---

## 10.4 OSC 133 semantic prompt + OSC 633 VS Code shell integration

**File(s):** `oriterm_core/tests/spec_chain/osc/shell_integration.rs` (new)

OSC 133 has 4 sub-commands (A=prompt start, B=command start, C=output start, D=command end). Each one triggers a state change in the term's prompt tracking + emits an event.

- [ ] Spec_chain test for OSC 133;A: emit, assert `prompt_mark_pending` becomes true on the term, assert (after the parser flushes) the prompt marker is recorded.
- [ ] Spec_chain test for OSC 133;B: assert `command_start_mark_pending` becomes true, assert the command_start row is recorded.
- [ ] Spec_chain test for OSC 133;C: assert `output_start_mark_pending` becomes true, assert the output_start row is recorded.
- [ ] Spec_chain test for OSC 133;D: assert `Effect::Host(HostEffect::CommandComplete { duration })` is emitted with a duration computed from when `OSC 133;C` was received.
- [ ] Spec_chain test for OSC 633 (VS Code) variants: research the VS Code shell integration source for the exact OSC 633 commands; write tests for each.
- [ ] Update `catalog/shell-integration.md` rows to `verified`.
- [ ] **Validation**: all OSC 133 / 633 tests pass; existing tack tests for shell integration still pass (if any).

---

## 10.5 OSC 22/50 cursor icon/shape, OSC 104/110/111/112 color reset

**File(s):** `oriterm_core/tests/spec_chain/osc/cursor_and_color_reset.rs` (new)

- [ ] OSC 22 cursor icon: emit `OSC 22 ; CursorShape=block ST`, assert state mutation
- [ ] OSC 50 cursor shape (legacy): same
- [ ] OSC 104 palette reset (all): emit `OSC 104 ST`, assert all palette indices restored to default
- [ ] OSC 104 palette reset (specific indices): emit `OSC 104 ; 5 ; 10 ST`, assert only those indices reset
- [ ] OSC 110/111/112 default fg/bg/cursor reset
- [ ] Update catalog rows to `verified`.
- [ ] **Validation**: tests pass.

---

## 10.6 OSC 1337 minimal subset (non-image variants)

**File(s):** `oriterm_core/tests/spec_chain/osc/iterm2_minimal.rs` (new)

OSC 1337 has many sub-commands; the image protocol (`File=...`) is the most complex and is deferred to section 14 (iTerm2 inline images). This subsection covers the simpler variants: SetMark, RemoteHost, CurrentDir, ShellIntegrationVersion.

- [ ] Read iTerm2 docs for the OSC 1337 minimal subset
- [ ] Spec_chain test for each variant
- [ ] Update `catalog/osc.md` rows for OSC-1337-SETMARK, OSC-1337-REMOTEHOST, OSC-1337-CURRENTDIR, OSC-1337-SHELLINTVERSION to `verified`.
- [ ] **Validation**: tests pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 10.4–10.6 (shell integration + cursor + minimal iterm2).

---

## 10.R Third Party Review Findings

- None.

---

## 10.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: OSC number × sub-command × apex layer (state, effect transcript, PTY reply)
- [ ] **Semantic pin**: OSC 8 hyperlink survives reflow/scroll test; OSC 52 ResponseToken round-trip test
- [ ] All OSC catalog rows beyond baseline are `verified`
- [ ] OSC 8 hyperlinks survive every grid transformation
- [ ] OSC 52 clipboard: store + load round-trip verified
- [ ] OSC 9/99/777 notification source discriminators verified
- [ ] OSC 133 semantic prompt + OSC 633 VS Code shell integration verified
- [ ] OSC 22/50 cursor icon/shape verified
- [ ] OSC 104/110/111/112 color reset verified
- [ ] OSC 1337 minimal subset (non-image) verified
- [ ] All existing teseq OSC tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 10 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every OSC catalog row is `verified`. The OSC suite is conformance-complete.
