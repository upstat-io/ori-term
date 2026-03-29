# Verify Roadmap Command

Systematically verify AND expand roadmap sections using parallel subagents. Two-phase process: (1) review agents audit existing items and identify gaps against each section's stated mission, (2) update agents apply findings back into the section files — which is the entire point of this command.

## Usage

```
/verify-roadmap [section]
```

- No args: Start from Section 1, Item 1
- `section-4`, `4`: Start from Section 4
- `continue`: Resume from last verified item (if tracking exists)

---

## Core Principle

**Verify what exists. Expand for what's missing. Write it all back.**

This command has two jobs:

1. **Verify** — Audit existing items: do tests pass, are they correct, is coverage adequate, do semantic pins exist?
2. **Expand** — Compare what the section *lists* against what it would *actually take* to fulfill the section's stated goal/mission. Identify missing functionality, missing tests, and missing items. A section that checks every listed box but doesn't achieve its stated goal is incomplete.

For each existing item:
- **Can verify -> verified** — Tests pass, feature works, coverage adequate -> mark `[x]`
- **Cannot verify -> annotate + pending** — Insufficient tests, missing coverage, no semantic pins -> add concrete test tasks, leave `[ ]`
- **Reopen if needed** — A previously-completed `[x]` item that fails coverage/pin checks gets reopened to `[ ]` with specific missing test tasks

For the section as a whole:
- **Read the section's stated goal/mission** — the title, description, and any goal statements
- **Ask: if every listed item were completed, would the goal be fulfilled?** If not, identify what's missing
- **Add missing items** — new `- [ ]` entries for functionality, tests, or work needed to actually achieve the goal
- **Never fix code, never write features** — only verify status, annotate deficiencies, and expand the plan

---

## Workflow

### Architecture: Two-Phase — Review Then Update

The command runs in two distinct phases:

**Phase 1 — Review**: Parallel subagents audit sections and write findings to temp files. Read-only — they do NOT modify section files.

**Phase 2 — Update**: After ALL reviews complete, a second set of parallel agents takes the review findings and applies them to the actual section files. This is the deliverable.

```
Main Context (Supervisor)
|
+-- PHASE 1: REVIEW (read-only)
|   +-- Batch 1: Launch review agents for sections A, B, C (in background)
|   |   +-- Agent: section-01 -> writes findings to .verify-results/section-01-results.md
|   |   +-- Agent: section-02 -> writes findings to .verify-results/section-02-results.md
|   |   +-- Agent: section-03 -> writes findings to .verify-results/section-03-results.md
|   +-- Monitor: Verify agents loaded context, audited tests, assessed coverage/pins
|   +-- Batch 2: Next batch of review agents...
|   +-- All reviews collected
|
+-- PHASE 2: UPDATE (writes section files)
|   +-- Launch update agents in parallel (one per section with findings)
|   |   +-- Agent: reads section-01-results.md -> updates section-01-*.md
|   |   +-- Agent: reads section-02-results.md -> updates section-02-*.md
|   |   +-- Agent: reads section-03-results.md -> updates section-03-*.md
|   +-- Each agent: applies status changes, annotations, expansions, new items
|   +-- Supervisor: validates updates, resolves conflicts
|
+-- Final: Update frontmatter, commit checkpoint
```

**Batch size (Phase 1)**: 3-4 sections per batch (avoids overwhelming system resources with concurrent `cargo test` runs).

**Phase 2 parallelism**: All update agents can run in parallel since they each write to different section files — no test conflicts.

### Phase 1: Review

#### Step 1: Plan Batches

Read all section files. Group into batches of 3-4 sections, ordered by section number. If the user specified a single section, skip batching — just run one agent.

#### Step 2: Launch Review Agent Batch

For each batch, launch parallel `general-purpose` subagents using the Agent tool with `run_in_background: true`. Each agent receives:

1. The section file path
2. Instructions to follow the verification protocol below
3. A results output path: `plans/roadmap/.verify-results/section-XX-results.md`

Each agent processes its section items sequentially (items within a section stay sequential to avoid test conflicts).

**MANDATORY: Every agent MUST begin by reading ALL project context.** Before verifying a single item, each agent must read — in full, every line — the following files:

1. `/home/eric/projects/ori_term/CLAUDE.md` — project instructions (read ALL of it)
2. Every file in `/home/eric/projects/ori_term/.claude/rules/` — ALL rules files, every line
3. The reference repos or standards relevant to the section being verified

Include this as an explicit instruction in each agent's prompt:
```
BEFORE YOU START: Read these files in full — every single line, no skipping.
Do not start verifying items until you have read ALL of these files.

1. /home/eric/projects/ori_term/CLAUDE.md (ALL of it — contains coding standards,
   performance invariants, testing requirements, crate boundaries)

2. ALL rules files in /home/eric/projects/ori_term/.claude/rules/ — read every file,
   every line:
   - code-hygiene.md, crate-boundaries.md, impl-hygiene.md, test-organization.md

3. The relevant reference repos for the section being verified (from
   ~/projects/reference_repos/console_repos/ — alacritty, wezterm, ghostty, etc.)

These files contain CRITICAL context: crate boundary rules, test organization standards,
performance invariants, coding standards, and hygiene rules.
An agent that skips reading these files WILL produce incorrect verification results.

After reading, report what you loaded at the top of your results file:
  Context loaded: CLAUDE.md (read), rules/*.md (4 files read), reference: [repos consulted]
```

This is non-negotiable. An agent that skips reading these files will miss critical context about crate boundaries, test organization, and performance invariants. The supervisor MUST verify that agent results begin with the "Context loaded" line showing all files were read. If the line is missing or shows fewer than 4 rules files, the agent's results are unreliable — re-run the section.

#### Step 3: Supervisor Monitoring

While review agents run, the main context:

1. **Periodically checks agent output** using Read on the output files
2. **Verifies agents loaded full context** — look for "Context loaded: CLAUDE.md (read), rules/*.md (N files read)" at the top of results. If missing, the agent skipped context loading and its results are unreliable — re-run the section.
3. **Verifies agents are actually reading tests** — look for evidence of file reads, not just "tests pass"
4. **Verifies agents assess coverage** — look for "Coverage assessment" blocks with relevant dimensions. An agent that marks items verified without coverage assessment is REJECTED.
5. **Verifies agents check for semantic pins** — look for "Semantic pin:" lines. An agent that marks items verified without identifying a pin is REJECTED.
6. **Verifies agents performed gap analysis** — look for "Gap analysis" block. An agent that doesn't analyze whether listed items fulfill the section's stated goal is INCOMPLETE.
7. **Verifies agents performed hygiene audit** — look for "Hygiene audit" blocks with file reads and rule checks. An agent that marks items verified without any hygiene assessment of implementation code is INCOMPLETE.
8. **Verifies agents performed protocol accuracy checks** — for any section touching terminal protocols, look for "Protocol verification" blocks with reference repo file:line citations. An agent that marks a protocol-touching item verified without showing cross-reference against at least TWO reference implementations is REJECTED.
9. **Flags agents that appear to skip any of the above** — if an agent marks items verified without showing context loading, test reads, coverage assessment, protocol verification (when applicable), pin identification, gap analysis, AND hygiene audit, intervene and re-verify
10. **Collects completed results** as agents finish

#### Step 4: Next Batch or Collect

If more review batches remain, go to Step 2. Otherwise, proceed to Phase 2.

**Do NOT apply findings to section files during Phase 1.** Review agents are read-only. All findings accumulate in `.verify-results/` temp files.

---

### Phase 2: Update Section Files

**This is the entire point of the command.** After ALL Phase 1 review agents have completed, launch update agents to write findings back into the actual roadmap section files.

#### Step 5: Launch Update Agents

For each section that has a results file in `.verify-results/`, launch a parallel `general-purpose` agent. Each update agent receives:

1. The review results file: `plans/roadmap/.verify-results/section-XX-results.md`
2. The section file to update: `plans/roadmap/section-XX-*.md`
3. Instructions to apply ALL findings from the review

All update agents can run in parallel — they each modify a different section file, so there are no conflicts.

**Each update agent MUST:**

1. **Read the review results file** in full
2. **Read the current section file** in full
3. **Apply every finding** from the review:
   - Update checkbox status (`[x]` / `[ ]`) based on verification results
   - Add annotations (PROTOCOL ERROR, PROTOCOL MISMATCH, INCOMPLETE PROTOCOL COVERAGE, WEAK TESTS, INCOMPLETE COVERAGE, NEEDS PIN, WRONG TEST, STALE TEST, NEEDS TESTS, BUG FOUND, REGRESSION, HYGIENE VIOLATION, HYGIENE NOTE, RULES VIOLATION) with specific sub-items — protocol annotations are listed FIRST
   - Reopen previously-completed `[x]` items that failed coverage/pin checks
   - **Add new `- [ ]` items** for missing functionality identified in gap analysis
   - **Add new `- [ ]` test items** for missing test coverage
   - Add verification dates on verified items: `(verified YYYY-MM-DD)`
4. **Update frontmatter** — status, last_verified date
5. **Preserve existing structure** — don't reorder sections, don't remove items, don't change item wording (only status and annotations)

#### Step 6: Supervisor Validates Updates

After update agents complete, the supervisor:

1. Reads each updated section file
2. Validates that all findings from the results file were applied
3. Checks frontmatter consistency (see TPR consistency checks below)
4. Reports the final summary

#### Step 7: Commit Checkpoint

After validation, commit all section file changes. Report summary to user.

---

## Phase 1: Review Agent Protocol

Each review subagent follows this protocol for its assigned section. Results go to `.verify-results/` — agents do NOT modify section files.

### For Each Item (Sequential within agent)

#### 2a. Identify Verification Method

For each item, determine how to verify it:

| Item Type | Verification Method |
|-----------|---------------------|
| `**Implement**: X` | Find and run related Rust tests |
| `**Rust Tests**: path` | Check if Rust tests exist at path, run them |
| `**Integration Test**: X` | Run integration tests |
| `**Architecture**: X` | Inspect code structure, verify pattern |
| Generic checkbox | Context-dependent verification |

#### 2b. Find and Run Tests

1. **Find related tests**:
   - Search `tests.rs` sibling files in the relevant crate
   - Search `tests/` directories for integration tests
   - Check architecture tests in `oriterm/tests/architecture.rs`

2. **Run tests** (ALWAYS with timeout):
   ```bash
   # For specific crate tests
   timeout 150 cargo test -p oriterm_core -- module_name

   # For UI crate tests
   timeout 150 cargo test -p oriterm_ui -- widget_name

   # For all tests
   timeout 150 ./test-all.sh
   ```

3. **Evaluate result**:
   - Tests exist AND pass -> proceed to **2c. Audit Test Quality**
   - Tests exist but fail -> **Not verified** (regression)
   - No tests exist -> **Cannot verify**

#### 2c. Audit Test Quality

**Every test that passes must be explicitly read and audited.** A passing test is NOT sufficient for verification — the test itself must be correct AND have adequate coverage. For each test file found:

1. **Read the test code** — Open and read every test. No exceptions, no skipping.

2. **Verify correctness against reference behavior**:
   - Does each assertion match expected terminal behavior (VT100/xterm/ECMA-48)?
   - Are expected values correct (not just copied from current output)?
   - Do error tests assert the right error type/message?

3. **Check for test quality issues**:
   - **False positives**: Tests that pass for the wrong reason (e.g., asserting `Ok(_)` without checking the value)
   - **Tautological tests**: Tests that can never fail (e.g., testing that `true == true`)
   - **Wrong assertions**: Expected values that don't match what the spec requires
   - **Missing coverage**: The feature has 5 behaviors but only 1 is tested
   - **Overly broad assertions**: `assert!(result.is_ok())` instead of checking the actual value
   - **Copy-paste errors**: Tests that are duplicates or test the wrong feature
   - **Stale tests**: Tests that reference outdated APIs or removed features

4. **Assess coverage** (see **2c-coverage** below)

5. **Verify protocol accuracy** (see **2c-proto** below)

6. **Check for semantic pins** (see **2c-pins** below)

7. **Classify the test quality**:

   | Quality | Meaning | Action |
   |---------|---------|--------|
   | **Sound** | Tests correct, assertions valid, coverage adequate, protocols accurate, pins exist | Mark `[x]` |
   | **Weak** | Tests pass but coverage insufficient, assertions shallow | Leave `[ ]`, annotate with specific gaps |
   | **No Coverage** | Tests pass for some scenarios but important dimensions untested | Leave `[ ]`, annotate as INCOMPLETE COVERAGE |
   | **Protocol Error** | Code references a protocol (OSC, CSI, Kitty, etc.) with wrong params, wrong sequence, or wrong behavior | Leave `[ ]`, annotate as PROTOCOL ERROR (critical, blocks verification) |
   | **No Pin** | Tests pass but no semantic pin exists — regression could go undetected | Leave `[ ]`, annotate as NEEDS PIN |
   | **Wrong** | Tests have incorrect assertions or test wrong behavior | Leave `[ ]`, annotate as WRONG TEST |
   | **Stale** | Tests reference outdated APIs/features | Leave `[ ]`, annotate as STALE TEST |

#### 2c-coverage. Coverage Assessment

**Every feature that touches shared code paths MUST have adequate test coverage.** A test suite that only exercises one scenario through a code path is incomplete, even if it passes.

For each item, identify the **coverage dimensions** that apply:

**Platform dimension** — Does behavior differ across platforms?
- Windows (ConPTY)
- macOS (PTY)
- Linux (PTY)
- Cross-platform (pure logic, no platform-specific code)

**Input dimension** — Which inputs exercise this code path?
- ASCII text
- Unicode (CJK, emoji, combining marks, ZWJ sequences)
- Control sequences (CSI, OSC, DCS, ESC)
- Edge cases: empty, single-char, maximum-length, boundary conditions

**Grid/terminal dimension** — Which terminal states matter?
- Normal mode
- Alternate screen
- Scrollback region
- Origin mode, wraparound mode
- With/without selection active
- With/without display offset (scrolled back)

**Widget/UI dimension** (for oriterm_ui items):
- Interaction states: idle, hovered, active/pressed, focused, disabled
- Input types: mouse, keyboard, touch
- Layout: minimum size, normal, overflow, resize
- Accessibility: focus order, keyboard navigation

**Rendering dimension** (for oriterm_gpu items):
- Cell types: normal, wide, combining, image
- Colors: 16-color, 256-color, truecolor, default
- Attributes: bold, italic, underline, strikethrough, inverse
- Special: cursor rendering, selection highlight, search highlight

Not every item needs all dimensions — identify which are **relevant** to the code path being verified. Use judgment — but err on the side of more coverage, not less.

**How to assess**: For each item, build a mental coverage grid:

```
Example: Grid reflow on resize
              | ASCII | CJK  | Emoji | Combining |
  shrink      |  ?    |  ?   |  ?    |    ?      |
  grow        |  ?    |  ?   |  ?    |    ?      |
  wrap point  |  ?    |  ?   |  ?    |    ?      |
  scrollback  |  ?    |  ?   |  ?    |    ?      |
  with select |  ?    |  ?   |  ?    |    ?      |
```

Fill in `[x]` for tested, `[ ]` for untested. If >30% of relevant cells are untested, classify as INCOMPLETE COVERAGE.

**Reporting coverage gaps**: When annotating, be explicit about which cells are missing:
```markdown
- INCOMPLETE COVERAGE: grid reflow — 4/20 cells covered
  - [ ] Add test: reflow with CJK characters (only ASCII tested)
  - [ ] Add test: reflow with emoji (wide chars at wrap boundary)
  - [ ] Add test: reflow shrink with active selection
  - [ ] Add test: reflow with scrollback content
  - [ ] Add test: reflow grow preserving cursor position
```

**Not all items need full coverage grids.** Simple items (e.g., "add CellFlags::BOLD") may only need a few tests. The coverage assessment scales with the complexity and breadth of the code path. Use judgment.

#### 2c-proto. Protocol Accuracy Verification — CRITICAL

**THIS IS THE HIGHEST-PRIORITY VERIFICATION STEP.** Every roadmap item that references a terminal protocol, escape sequence, OSC command, CSI sequence, DCS sequence, Kitty protocol extension, iTerm2 protocol, xterm extension, or any other standardized terminal behavior MUST be verified for 100% accuracy against the authoritative specification AND against reference implementations.

**A protocol error is worse than a missing feature.** Wrong protocol behavior causes silent data corruption, garbled output, broken interop with other tools, and user-visible rendering bugs that are extremely hard to diagnose. There is zero tolerance for protocol inaccuracy.

##### What Triggers Protocol Verification

Any item that mentions, implements, or tests any of the following:

| Protocol Family | Examples | Authoritative Source |
|-----------------|----------|---------------------|
| **ECMA-48 / ANSI** | CSI sequences, SGR attributes, cursor movement, scrolling, erase, modes | ECMA-48 standard |
| **xterm extensions** | Private modes (DECSET/DECRST), mouse tracking, bracketed paste, synchronized output, focus events | xterm ctlseqs (invisible-island.net) |
| **VT100/VT220/VT320/VT520** | DEC private modes, DECOM, DECAWM, DECCOLM, DECSC/DECRC, DECSTBM | DEC terminal manuals |
| **OSC (Operating System Commands)** | OSC 0-2 (title), OSC 4 (color), OSC 7 (CWD), OSC 8 (hyperlinks), OSC 10-19 (dynamic colors), OSC 52 (clipboard), OSC 104 (reset color), OSC 112 (reset cursor color), OSC 133 (shell integration / prompt marking) | xterm ctlseqs + individual protocol docs |
| **Kitty graphics protocol** | Image display (APC + `_G`), placement, animation, Unicode placeholders, transmission modes (direct, file, temp file, shared memory), image formats, composition, z-index | Kitty documentation (sw.kovidgoyal.net) |
| **Kitty keyboard protocol** | Progressive enhancement flags, CSI u sequences, key reporting modes, associated text, modifier encoding, disambiguate/report-events/report-alternates/report-all-keys-as-escape-codes/report-associated-text flags | Kitty documentation |
| **iTerm2 protocol** | Inline images (OSC 1337), shell integration marks, custom escape sequences, badges, annotations, notifications, profile switching, file transfer | iTerm2 documentation |
| **sixel** | DCS + sixel data, color registers, raster attributes, geometry | VT340 manual + libsixel |
| **DCS (Device Control String)** | DECRQSS, XTGETTCAP, sixel, ReGIS | DEC manuals + xterm ctlseqs |
| **SGR (Select Graphic Rendition)** | Attributes 0-9, 21-29, 30-37, 38, 39, 40-47, 48, 49, 53, 58, 59, 90-97, 100-107 | ECMA-48 + common extensions |
| **Mouse protocols** | X10, normal, button, any-event tracking; SGR, UTF-8, urxvt encoding | xterm ctlseqs |
| **Mode 2026** | Synchronized output (begin/end markers) | Terminal-wg spec |
| **Mode 2027** | Grapheme cluster mode | Terminal-wg spec |
| **Mode 2031** | Colored underlines | Proposal spec |
| **XTVERSION** | CSI > 0 q terminal identification | xterm ctlseqs |
| **DA1/DA2/DA3** | Device attributes (primary, secondary, tertiary) | VT220+ manuals |

##### How to Verify Protocol Accuracy

For EVERY protocol reference in a roadmap item:

**Step 1 — Identify the exact sequence/command.** Extract the literal byte sequence, parameter positions, parameter types, and expected terminal behavior.

**Step 2 — Cross-reference against the authoritative spec.** Do NOT trust comments in the code or descriptions in the roadmap — verify against the actual specification:

```
For CSI sequences:   Read xterm ctlseqs — find the EXACT CSI entry
For OSC sequences:   Read xterm ctlseqs — find the EXACT OSC number and parameter format
For Kitty protocol:  Read ~/projects/reference_repos/console_repos/kitty/ source
                     AND Kitty docs (check keyboard.py, graphics.py)
For DEC modes:       Read VT220/VT320/VT520 manual entries
For SGR attributes:  Read ECMA-48 section 8.3.117 + common terminal extensions
For mouse protocols: Read xterm ctlseqs mouse tracking section
```

**Step 3 — Cross-reference against reference implementations.** Check how the major terminals actually implement it — not just what the spec says, but what the ecosystem expects:

| Reference Repo | What to Check | Path Pattern |
|---------------|---------------|-------------|
| **alacritty** | VTE handler dispatch, CSI/OSC/DCS handlers, mode flags | `alacritty_terminal/src/term/mod.rs`, `event.rs` |
| **wezterm** | Comprehensive protocol support, CSI/OSC dispatch, Kitty keyboard, Kitty graphics, sixel, iTerm2 images | `term/src/terminalstate/`, `termwiz/src/escape/` |
| **ghostty** | SIMD-optimized VT parser, mode handling, Kitty keyboard, Kitty graphics, sixel | `src/terminal/`, `src/termio/` |
| **kitty** | THE authoritative source for Kitty protocols, keyboard protocol, graphics protocol | `kitty/`, `kittens/`, `docs/` |
| **tmux** | Input parser (the 83k-line `input.c`), CSI/OSC handling, screen operations | `input.c`, `screen.c`, `tty.c` |
| **foot** | Clean Wayland terminal, good protocol conformance | `terminal.c`, `vt.c` |

**Step 4 — Verify parameter encoding.** The most common protocol bugs are:

- **Wrong parameter separator**: `;` vs `:` (SGR colon-separated subparams for underline colors use `:`, e.g., `\e[58:2::R:G:Bm`, NOT `\e[58;2;R;G;Bm`)
- **Wrong parameter base**: decimal vs hex vs octet (OSC 4 colors use `rgb:RR/GG/BB` or `#RRGGBB`, not raw integers)
- **Wrong default values**: Many CSI params default to 0 or 1 — know which (CSI A default is 1, not 0)
- **Wrong parameter count**: Some sequences have optional trailing params (SGR 38/48 have `2;r;g;b` AND `5;idx` forms)
- **Wrong terminator**: OSC terminates with ST (`\e\\` or `\x9c`) or BEL (`\x07`) — both must be accepted
- **Wrong introducer**: CSI is `\e[` (7-bit) or `\x9b` (8-bit), OSC is `\e]` or `\x9d`, DCS is `\eP` or `\x90`
- **Missing intermediate bytes**: Some sequences have intermediate bytes (`\e[?` for DEC private modes, `\e[>` for DA2 response)
- **Conflated sequences**: CSI `h`/`l` (SM/RM) vs CSI `?h`/`?l` (DECSET/DECRST) are DIFFERENT — never mix them
- **Wrong state handling**: Some modes are per-screen (saved/restored with alt screen), others are global

**Step 5 — Build a protocol test matrix.** Every protocol-touching item gets a protocol-specific test matrix:

```
Example: OSC 52 (clipboard access)
                        | base64 data | empty data | invalid base64 | selection target |
  set clipboard (c)     |     ?       |     ?      |      ?         |       ?          |
  set primary (p)       |     ?       |     ?      |      ?         |       ?          |
  set both (pc)         |     ?       |     ?      |      ?         |       ?          |
  query clipboard (?)   |     ?       |     ?      |      ?         |       ?          |
  ST terminator         |     ?       |            |                |                  |
  BEL terminator        |     ?       |            |                |                  |

Example: Kitty keyboard protocol
                              | flag 1  | flag 2  | flag 4  | flag 8  | flag 16 | combined |
  push mode (CSI > u)         |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  pop mode (CSI < u)          |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  query mode (CSI ? u)        |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  key event (CSI num ; mod u) |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  functional keys             |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  modifier-only events        |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  text-as-codepoints          |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |
  associated text             |    ?    |    ?    |    ?    |    ?    |    ?    |    ?     |

Example: SGR attributes
                  | enable | disable | reset-all | with truecolor | with 256color |
  bold (1)        |   ?    |    ?    |     ?     |       ?        |       ?       |
  dim (2)         |   ?    |    ?    |     ?     |       ?        |       ?       |
  italic (3)      |   ?    |    ?    |     ?     |       ?        |       ?       |
  underline (4)   |   ?    |    ?    |     ?     |       ?        |       ?       |
  blink (5)       |   ?    |    ?    |     ?     |       ?        |       ?       |
  inverse (7)     |   ?    |    ?    |     ?     |       ?        |       ?       |
  invisible (8)   |   ?    |    ?    |     ?     |       ?        |       ?       |
  strikethrough(9)|   ?    |    ?    |     ?     |       ?        |       ?       |
  underline color |   ?    |    ?    |     ?     |       ?        |       ?       |
  overline (53)   |   ?    |    ?    |     ?     |       ?        |       ?       |
```

**Every cell in the protocol test matrix must be filled.** Protocol matrices have a ZERO tolerance threshold — not the 30% threshold used for general coverage. If even one relevant cell is untested, classify as INCOMPLETE PROTOCOL COVERAGE and add specific test items.

##### Protocol Accuracy Annotations

**PROTOCOL ERROR** (Critical — blocks verification, higher severity than HYGIENE VIOLATION):
```markdown
- [ ] **Implement**: Kitty keyboard protocol
  - PROTOCOL ERROR: Wrong modifier encoding — code uses `1 + modifiers` but Kitty spec
    uses `modifier_flags + 1` where flags are: shift=1, alt=2, ctrl=4, super=8, hyper=16, meta=32
    - Reference: kitty/keys.py:encode_key_event()
    - Reference: alacritty_terminal/src/term/mod.rs (CSI u handling)
    - [ ] Fix modifier encoding to match Kitty spec exactly
    - [ ] Add pin test: Ctrl+A sends CSI 97;5u (not CSI 97;4u)
```

**PROTOCOL MISMATCH** (Critical — our behavior differs from what all reference impls agree on):
```markdown
- [ ] **Implement**: OSC 4 color query
  - PROTOCOL MISMATCH: Response format wrong — our code responds with `rgb:RRRR/GGGG/BBBB`
    (16-bit per channel) but xterm/alacritty/wezterm all respond with `rgb:RR/GG/BB` when
    the original set used 8-bit values
    - Reference: xterm ctlseqs, OSC 4 response format
    - Reference: wezterm/term/src/terminalstate/performer.rs (osc_response)
    - [ ] Fix response to match reference implementations
    - [ ] Add pin test: query color 0 after setting to `rgb:ff/00/00` returns same format
```

**INCOMPLETE PROTOCOL COVERAGE** (Critical — protocol tests must be exhaustive):
```markdown
- [ ] **Implement**: Mouse tracking modes
  - INCOMPLETE PROTOCOL COVERAGE: SGR mouse encoding — 4/16 cells tested
    - [ ] Add test: button press (M=0) with SGR encoding `CSI < 0 ; x ; y M`
    - [ ] Add test: button release with SGR encoding `CSI < 0 ; x ; y m` (lowercase m!)
    - [ ] Add test: mouse move with button held `CSI < 32 ; x ; y M`
    - [ ] Add test: scroll up `CSI < 64 ; x ; y M`
    - [ ] Add test: scroll down `CSI < 65 ; x ; y M`
    - [ ] Add test: coordinates > 223 (SGR handles this, X10/normal do NOT)
    - [ ] Add test: modifier keys in mouse events (shift=4, meta=8, ctrl=16 added to button)
    - Pin: reference alacritty mouse.rs, wezterm mouse.rs for exact encoding
```

##### Protocol Verification in Reference Repos — MANDATORY

**Agents MUST actually read the reference repo source code** for every protocol item. Not skim — READ. The agent prompt must include:

```
For EVERY protocol-related item, you MUST:

1. Read our implementation code (the actual byte sequences we emit/parse)
2. Read at least TWO reference implementations for the same protocol:
   - ~/projects/reference_repos/console_repos/alacritty/ — check term/mod.rs, event.rs
   - ~/projects/reference_repos/console_repos/wezterm/ — check term/src/terminalstate/
   - ~/projects/reference_repos/console_repos/ghostty/ — check src/terminal/
   - ~/projects/reference_repos/console_repos/kitty/ — check kitty/ (authoritative for Kitty protocols)
   - ~/projects/reference_repos/console_repos/tmux/ — check input.c
3. Compare byte-for-byte: parameter order, separator characters, default values,
   terminator handling, mode flags, response format
4. If our code disagrees with ALL reference implementations, it's PROTOCOL ERROR
5. If our code disagrees with SOME reference implementations, note which agree/disagree
   and identify the authoritative spec to resolve
6. Report specific file:line references in BOTH our code AND the reference repos

DO NOT mark any protocol-touching item as verified without showing evidence of
reference repo comparison. A protocol item marked verified without reference
cross-check is AUTOMATICALLY REJECTED by the supervisor.
```

##### Protocol Semantic Pins — Every Protocol Gets One

Every protocol implementation MUST have at least one semantic pin that verifies the exact byte sequence. Not "does it parse" — "does it produce/consume the exact bytes the spec requires."

**Good protocol pins:**
```rust
// Pin: CSI 38;2;R;G;B m sets truecolor foreground
// Verifies exact parameter order (2=truecolor, then R, G, B)
handler.input(b"\x1b[38;2;255;128;0m");
assert_eq!(cell.fg, Color::Rgb(Rgb { r: 255, g: 128, b: 0 }));

// Pin: Kitty keyboard Ctrl+A produces CSI 97 ; 5 u
// Verifies modifier encoding (ctrl=4, +1 = 5) and key codepoint (a=97)
let output = terminal.key_input(Key::A, Modifiers::CTRL);
assert_eq!(output, b"\x1b[97;5u");

// Pin: OSC 52 clipboard set uses base64, accepts both ST and BEL terminator
handler.input(b"\x1b]52;c;SGVsbG8=\x1b\\");  // ST terminator
assert_eq!(clipboard.get("c"), Some("Hello"));
handler.input(b"\x1b]52;c;V29ybGQ=\x07");     // BEL terminator
assert_eq!(clipboard.get("c"), Some("World"));

// Pin: SGR underline color uses COLON separators, not semicolons
handler.input(b"\x1b[58:2::255:0:0m");  // colon-separated, correct
assert_eq!(cell.underline_color, Some(Color::Rgb(Rgb { r: 255, g: 0, b: 0 })));
```

**Bad protocol pins (REJECTED):**
```rust
// BAD: Doesn't verify the actual bytes, just that "something happened"
handler.input(some_csi_sequence);
assert!(cell.fg != Color::default());

// BAD: Tests parsing but not the exact parameter interpretation
assert!(parse_csi(b"\x1b[38;2;255;0;0m").is_ok());
```

#### 2c-pins. Semantic Pin Verification

**Every non-trivial feature MUST have at least one semantic pin test** — a test that ONLY passes with the correct implementation and would FAIL if the feature were reverted, removed, or incorrectly implemented.

A semantic pin is NOT:
- `assert_eq!(grid.cols(), 80)` — this tests the default, not a specific feature
- `assert!(result.is_ok())` — this is too broad; many wrong implementations also return Ok
- A test that could pass with a stub implementation

A semantic pin IS:
- A test that asserts the **specific** output/behavior that distinguishes this implementation from a naive/wrong/missing one
- A test that would fail if you commented out the feature's implementation
- A test that verifies an edge case that only the correct algorithm handles

**How to assess**: For each verified item, ask: "If I reverted the implementation commit for this feature, would at least one test fail with a *meaningful* error that identifies the regression?" If the answer is no, it needs a pin.

**Reporting missing pins**:
```markdown
- NEEDS PIN: grid.scroll_up() — tests exist but all would pass even with a no-op scroll
  - [ ] Add semantic pin: scroll_up(1) moves row 0 content to scrollback, row 1 becomes row 0
```

#### 2d. Update Item Status

**If Verified (tests pass, sound, coverage adequate, pins exist):**
```markdown
- [x] **Implement**: Feature X [done] (verified 2026-03-29)
```

**If Not Verified (regression — tests fail):**
```markdown
- [ ] **Implement**: Feature X
  - REGRESSION: Tests exist but fail. Needs investigation.
```

**If Tests Weak (pass but insufficient):**
```markdown
- [ ] **Implement**: Feature X
  - WEAK TESTS: Tests pass but coverage is insufficient
    - [ ] Add test: [specific missing coverage]
    - [ ] Strengthen assertion in [test file]: assert actual value, not just Ok
```

**If Coverage Incomplete (tests pass for some scenarios but not all relevant ones):**
```markdown
- [ ] **Implement**: Feature X
  - INCOMPLETE COVERAGE: [N]/[M] cells covered — missing [specific dimensions]
    - [ ] Add test: [feature] with [missing input] (only [tested input] covered)
    - [ ] Add test: [feature] with [missing state] (e.g., alternate screen, scrolled back)
    - [ ] Add test: [feature] with [missing interaction] (e.g., resize during operation)
```

**If No Semantic Pin (tests pass but no regression guard):**
```markdown
- [ ] **Implement**: Feature X
  - NEEDS PIN: Tests exist but none would uniquely fail if feature reverted
    - [ ] Add semantic pin: [specific assertion that only correct implementation satisfies]
```

**If Tests Wrong (incorrect assertions):**
```markdown
- [ ] **Implement**: Feature X
  - WRONG TEST: [test file] — [what's wrong]
    - Expected per reference: [correct behavior]
    - Test asserts: [what test currently checks]
```

**If Tests Stale (outdated APIs/features):**
```markdown
- [ ] **Implement**: Feature X
  - STALE TEST: [test file] — references removed/changed API
```

**If Cannot Verify (no tests):**
```markdown
- [ ] **Implement**: Feature X
  - NEEDS TESTS: Add verification tests before marking complete
    - [ ] Add test: [specific test description]
    - [ ] Add test: [edge case description]
```

**If Hygiene Violation (critical — blocks verification):**
```markdown
- [ ] **Implement**: Feature X
  - HYGIENE VIOLATION: LEAK — [file:line] [description of side logic / phase bleeding]
    - [ ] [Concrete remediation action]
  - RULES VIOLATION: CLAUDE.md — [file:line] [specific rule violated]
    - [ ] [Concrete remediation action]
```

**If Hygiene Note (informational — does not block verification):**
```markdown
- [x] **Implement**: Feature X [done] (verified 2026-03-29)
  - HYGIENE NOTE: BLOAT — [file] is [N] lines (limit 500)
    - [ ] Split into submodules
  - HYGIENE NOTE: WASTE — unnecessary .clone() at [file:line]
```

#### 2d-reopen. Reopening Previously Completed Items

**A `[x]` item that fails coverage or pin checks MUST be reopened to `[ ]`.**

Previously-verified items are NOT exempt from coverage and pin requirements. If a section was marked complete but its tests lack coverage or semantic pins, the item is reopened and the section status changes accordingly.

When reopening:
1. Change `[x]` to `[ ]`
2. Remove any `[done]` or `(verified ...)` annotation
3. Add the specific deficiency annotation (INCOMPLETE COVERAGE, NEEDS PIN, etc.)
4. Add concrete `- [ ]` sub-items for each missing test
5. Update section frontmatter status from `complete` to `in-progress`

```markdown
# Before (previously verified):
- [x] **Implement**: grid.reflow() [done] (verified 2026-02-15)

# After (reopened — missing coverage):
- [ ] **Implement**: grid.reflow()
  - INCOMPLETE COVERAGE: 3/15 cells covered — only ASCII tested
    - [ ] Add test: reflow with CJK characters (width 2 at wrap boundary)
    - [ ] Add test: reflow with emoji (ZWJ sequences spanning wrap)
    - [ ] Add test: reflow with combining marks
    - [ ] Add test: reflow preserving selection coordinates
    - [ ] Add test: reflow with scrollback content
  - NEEDS PIN: no test uniquely identifies reflow vs. truncation
    - [ ] Add semantic pin: reflow shrink wraps long line, grow unwraps it back
```

**This is not punitive — it's protective.** An item without coverage is a future regression waiting to happen. Reopening ensures the test gaps are visible and tracked in the planning system, not buried as invisible assumptions.

#### 2e. Implementation Hygiene & Rules Audit

**After verifying tests, audit the implementation code for hygiene violations and CLAUDE.md / rules violations.** This catches architectural decay, coding standard drift, and rule violations in the code that implements each section's features.

For each `**Implement**` item, identify the source files that implement the feature (the agent already located these while finding tests). Read those files and check against:

1. **`impl-hygiene.md` rules** — the full hygiene ruleset. Key categories to check:
   - **LEAK**: Side logic, phase bleeding, duplicated dispatch, scattered knowledge, validation bypass
   - **DRIFT**: Registration data present in one location but missing from sync points
   - **GAP**: Feature supported in one layer but blocked/missing in another
   - **WASTE**: Unnecessary allocation, clone, or transformation at boundary
   - **EXPOSURE**: Internal state leaking through boundary types
   - **BLOAT**: File exceeds 500-line limit, mixes responsibilities

2. **CLAUDE.md coding guidelines** — the project-level rules:
   - **Crate boundaries**: no upward dependencies, correct ownership per crate
   - **Error handling**: no `unwrap()` in library code, `Result` or defaults
   - **Unsafe**: `unsafe_code = "deny"`, zero unsafe in library code
   - **API**: >3 params -> config struct, no boolean flags, `#[must_use]` on builders
   - **Style**: no `#[allow(clippy)]` without justification, functions < 50 lines, no dead/commented code
   - **File size**: 500-line limit (excl. tests), split before exceeding
   - **Memory**: newtypes for IDs, `Arc` only when shared ownership required, no alloc in hot paths
   - **Performance**: no O(n^2), hash lookups not linear scans, buffer output flush atomically
   - **Cross-platform**: every `#[cfg(target_os)]` must have counterparts for all 3 platforms

3. **`.claude/rules/*.md` domain-specific rules** — each section maps to domain rules:
   - Grid/cell sections -> `code-hygiene.md` rules
   - UI/widget sections -> `code-hygiene.md`, `crate-boundaries.md` rules
   - Mux/PTY sections -> `crate-boundaries.md` rules
   - Test sections -> `test-organization.md` rules
   - Cross-crate sections -> `crate-boundaries.md` rules
   - All sections -> `impl-hygiene.md` rules

**How to audit**: For each implementation file:
- Check file length (>500 lines = BLOAT)
- Check for `unwrap()` in library code (= ERROR HANDLING VIOLATION)
- Check for `unsafe` blocks (= UNSAFE VIOLATION)
- Check for bare `#[allow(clippy::...)]` without reason (= LINT VIOLATION)
- Check for dead code, commented-out code (= WASTE)
- Check for decorative banners `// ===`, `// ---` (= STYLE VIOLATION)
- Check for missing `///` docs on pub items (= DOC VIOLATION)
- Check function lengths (>50 lines = STYLE VIOLATION, >30 = NOTE)
- Check for `println!` debugging (= STYLE VIOLATION, use `log` macros)
- Check for allocations in hot render paths (= PERFORMANCE VIOLATION)
- Check for missing cross-platform counterparts (= PLATFORM VIOLATION)
- Check crate boundary violations (= ARCHITECTURE VIOLATION)

**Severity mapping**:
| Hygiene Category | Verification Annotation | Severity |
|------------------|------------------------|----------|
| Protocol error | PROTOCOL ERROR: [specific sequence/behavior] | CRITICAL (highest) |
| Protocol mismatch | PROTOCOL MISMATCH: [our behavior vs reference] | CRITICAL |
| Incomplete protocol coverage | INCOMPLETE PROTOCOL COVERAGE: [N/M cells] | CRITICAL |
| LEAK (any sub-type) | HYGIENE VIOLATION: LEAK | CRITICAL |
| DRIFT | HYGIENE VIOLATION: DRIFT | CRITICAL |
| GAP | HYGIENE VIOLATION: GAP | CRITICAL |
| WASTE | HYGIENE NOTE: WASTE | Informational |
| EXPOSURE | HYGIENE NOTE: EXPOSURE | Informational |
| BLOAT | HYGIENE VIOLATION: BLOAT | Informational |
| CLAUDE.md violation | RULES VIOLATION: [specific rule] | CRITICAL |
| Domain rule violation | RULES VIOLATION: [rule file]: [specific rule] | CRITICAL |

**Critical vs. informational**: PROTOCOL ERROR/MISMATCH (highest severity), LEAK, DRIFT, GAP, and CLAUDE.md violations are **critical** — they block verification (item stays `[ ]`). WASTE, EXPOSURE, and BLOAT are **informational** — noted but don't block verification. Protocol errors are listed FIRST in any findings report because they are the most dangerous class of defect.

**Reporting hygiene findings**:
```markdown
- HYGIENE VIOLATION: LEAK — oriterm_core/src/grid/mod.rs:245
  - Rendering logic mixed into grid data structure
  - [ ] Move rendering concern to oriterm_gpu
- HYGIENE VIOLATION: BLOAT — oriterm_ui/src/widgets/button/mod.rs is 680 lines
  - [ ] Split into submodules: button/layout.rs, button/paint.rs
- RULES VIOLATION: CLAUDE.md — unwrap() at oriterm_mux/src/pane/mod.rs:89
  - [ ] Replace with proper error handling or default
- RULES VIOLATION: crate-boundaries.md — oriterm_core imports from oriterm_ui
  - [ ] Move shared type to oriterm_core, remove upward dependency
```

**Scope**: Only audit files that implement features in the current section. Do NOT audit the entire codebase — this is section-scoped, not project-scoped. For a project-wide hygiene audit, use `/code-hygiene-review` or `/impl-hygiene-review` instead.

#### 2f. Section-Level Gap Analysis

**After verifying all individual items, step back and analyze the section as a whole.** This is where expansion happens.

1. **Re-read the section's title, description, and goal/mission statement** — what does this section claim to accomplish?

2. **Ask: "If every listed item (even the unchecked ones) were completed, would this section's stated goal be fulfilled?"**
   - If YES -> gap analysis complete, note "No gaps found"
   - If NO -> identify what's missing

3. **Identify missing items in these categories:**

   **MISSING FUNCTIONALITY** — features or capabilities not listed but required by the goal:
   ```markdown
   MISSING FUNCTIONALITY (gap analysis):
   - [ ] **Implement**: [feature] — required by section goal "[goal]" but not listed
   - [ ] **Implement**: [feature] — reference impl (alacritty/wezterm) has this but section omits it
   ```

   **MISSING TESTS** — test coverage that doesn't exist for any listed item:
   ```markdown
   MISSING TESTS (gap analysis):
   - [ ] **Rust Tests**: oriterm_core/src/grid/tests.rs — [what needs testing]
   - [ ] **Rust Tests**: oriterm_ui/src/widgets/button/tests.rs — [what needs testing]
   ```

   **MISSING ITEMS** — work items needed to bridge gaps (integration, wiring, etc.):
   ```markdown
   MISSING ITEMS (gap analysis):
   - [ ] Wire [feature X] to [feature Y] — both listed separately but integration not tracked
   - [ ] Add cross-platform handling for [edge case] — Windows/macOS behave differently here
   ```

4. **Check reference repos** — consult alacritty, wezterm, ghostty in `~/projects/reference_repos/console_repos/` for features that established terminal emulators implement but this section doesn't cover

5. **Check the codebase** — are there TODOs, FIXMEs, or `#[ignore]` markers in the code that relate to this section's domain but aren't tracked?

6. **Report gap analysis results:**
   ```
   Gap analysis for Section 5 — Window + GPU Rendering:
     Section goal: "GPU-accelerated terminal rendering with wgpu"
     Listed items cover: basic cell rendering, glyph atlas, cursor
     GAPS FOUND:
       - Ligature rendering not listed (wezterm/ghostty both support this)
       - Damage tracking not listed (performance invariant requires minimal GPU work)
       - Image protocol (sixel/kitty) rendering not tracked
       - 2 #[ignore] tests in oriterm_gpu/src/renderer/ reference untracked issues
       - 1 TODO in oriterm_gpu/src/atlas.rs:142 — "TODO: handle atlas overflow"
     Items to add: 3 functionality, 2 test, 1 integration
   ```

**This is the most important step.** A section where every checkbox is green but the stated goal isn't achievable is worse than one with honest gaps — it creates false confidence. Expand the section to match reality.

#### 2g. Report Progress

After all items + gap analysis, report per-item results and gap summary:
```
V 1.1 Cell struct with CellFlags — VERIFIED (5 tests, coverage 8/8, pin: bold_flag_set, hygiene: clean)
X 1.2 Unicode width calculation — INCOMPLETE COVERAGE (tests pass but only ASCII, 3/12 cells)
X 1.3 Grid scroll — WRONG TEST (asserts scroll_up returns (), should verify row content moved)
X 1.4 Selection coordinates — NEEDS TESTS (no tests found)
X 1.5 Reflow on resize — NEEDS PIN (8 tests pass but none uniquely identifies correct reflow)
X 1.6 Grid indexing — REOPENED (was [x], coverage 2/10 cells, no pin)
X 1.7 VTE handler — HYGIENE VIOLATION: LEAK (rendering logic in term_handler.rs)
H 1.8 Cell attributes — VERIFIED but HYGIENE NOTE: BLOAT (cell.rs is 520 lines)
+ GAP: 2 missing functionality items, 3 missing test items (see gap analysis)
```

### Frontmatter Updates

Phase 2 update agents apply frontmatter changes:
- All items `[x]` -> `status: complete`
- Mixed -> `status: in-progress`
- All items `[ ]` -> `status: not-started`
- **Any reopened items** -> section status MUST change to `in-progress` (even if it was `complete`)
- **Any new items added from gap analysis** -> section status MUST be `in-progress`

#### Third Party Review Consistency Checks

The supervisor must also validate `third_party_review` frontmatter consistency:

1. **`status: complete` + `third_party_review.status: findings`** = INVALID — a section cannot be complete with unresolved TPR findings. Set section `status` to `in-progress`.
2. **Unchecked TPR items exist + `third_party_review.status: none`** = INVALID — set `third_party_review.status` to `findings`.
3. **Unchecked TPR items exist + `third_party_review.status: resolved`** = INVALID — set `third_party_review.status` to `findings`.
4. **All TPR items checked + `third_party_review.status: findings`** = STALE — set `third_party_review.status` to `resolved`.
5. **No TPR block or empty (`- None.`) + `third_party_review.status: findings`** = INVALID — set `third_party_review.status` to `none`.

Report any TPR consistency fixes alongside normal frontmatter updates in the batch summary.

### Commit Checkpoint (Phase 2 only)

Commits happen after Phase 2 update agents have written findings into section files — never after Phase 1 alone.

```
Verification complete (Sections 1, 2, 3).
Phase 1 (review): All sections reviewed
Phase 2 (update): All section files updated

Section 1 — Cell + Grid:
  Verified: 18/25 | Reopened: 2 | New items from gap analysis: 3
Section 2 — Terminal State Machine + VTE:
  Verified: 30/38 | Reopened: 0 | New items from gap analysis: 2
Section 3 — Cross-Platform:
  Verified: 12/15 | Reopened: 1 | New items from gap analysis: 4
```

---

## Verification Criteria

### What Counts as "Verified"

ALL of the following must be true:

1. **Tests exist** — At least one test directly exercises the feature
2. **Tests pass** — All related tests pass (with timeout 150s)
3. **Tests are correct** — Every assertion has been READ and checked against expected behavior
4. **Tests have adequate coverage** — Happy path, edge cases, and error cases are covered
5. **Assertions are specific** — Tests check actual values, not just `is_ok()` / `is_some()`
6. **Coverage adequate** — All relevant dimensions (platform, input, state) are tested
7. **Protocol accuracy verified** — If the item touches any terminal protocol (CSI, OSC, DCS, Kitty, SGR, mouse, etc.), the exact byte sequences, parameter encoding, default values, and behavior have been cross-referenced against the authoritative spec AND at least two reference implementations. Protocol test matrix is 100% filled (zero tolerance — not the 30% threshold for general coverage)
8. **Semantic pin exists** — At least one test would uniquely fail if the feature were reverted. For protocol items, at least one pin verifies the exact byte sequence produced or consumed
9. **No critical hygiene violations** — Implementation code has no LEAK, DRIFT, GAP, PROTOCOL ERROR, or CLAUDE.md/rules violations (informational notes like BLOAT/WASTE are OK)

### What Counts as "Weak Tests"

1. **Shallow assertions** — `assert!(result.is_ok())` without checking the value
2. **Single path only** — Only happy path tested, no edge cases or errors
3. **Missing feature coverage** — Feature has 5 behaviors, tests cover 2

### What Counts as "Incomplete Coverage"

1. **Single-input coverage** — Tests only exercise ASCII through a path that handles Unicode
2. **Missing platform dimension** — Only tested on Linux, has `#[cfg]` blocks for Windows/macOS
3. **No terminal state coverage** — Only normal mode tested; alternate screen, origin mode, etc. skipped
4. **Missing edge-case dimension** — No empty grid, no single-cell, no boundary conditions
5. **Missing interaction dimension** — Only idle state tested; hover, pressed, focused untested
6. **Missing rendering dimension** — Only basic cells tested; wide chars, images, attributes skipped

**Threshold**: If >30% of relevant cells are untested, classify as INCOMPLETE COVERAGE.

### What Counts as "Protocol Error" (Critical — blocks verification, highest severity)

1. **Wrong byte sequence** — Code emits or parses bytes that don't match the authoritative spec (e.g., wrong CSI parameter separator, wrong OSC number, wrong DCS introducer)
2. **Wrong parameter encoding** — Parameters in wrong order, wrong type (decimal vs hex), wrong default values, wrong separator character (`;` vs `:`)
3. **Wrong terminator handling** — Not accepting both ST (`ESC \`) and BEL (`0x07`) for OSC, or using wrong terminator for DCS
4. **Wrong mode semantics** — DECSET/DECRST mode does the wrong thing, or conflates SM/RM with DECSET/DECRST
5. **Wrong response format** — DA1/DA2/DA3, DECRQSS, XTVERSION, color queries return wrong format
6. **Protocol conflation** — Mixing up two different protocols (e.g., Kitty keyboard CSI u vs xterm modifyOtherKeys)
7. **Missing protocol state** — Not saving/restoring mode state on alt screen switch when the protocol requires it
8. **Fabricated protocol** — Claiming to implement a protocol that doesn't exist or inventing non-standard extensions without documentation

**Protocol errors are NEVER informational.** Every protocol error blocks verification and requires explicit fix items. A single wrong byte in a CSI sequence can break every application that depends on it.

### What Counts as "Needs Pin"

1. **No regression guard** — All tests could pass with a trivially wrong implementation
2. **Too-broad assertions** — Tests check types or shapes but not specific values
3. **Redundant with simpler features** — Tests could pass if the feature delegated to a no-op
4. **Pin criteria**: At least one test must assert a **specific computed value** that only the correct implementation of this exact feature produces

### What Counts as "Wrong Tests"

1. **Incorrect expected values** — Assertion doesn't match expected terminal behavior
2. **Testing wrong behavior** — Test name says "scroll" but tests cursor movement
3. **Copy-paste errors** — Test is a duplicate of another with no meaningful difference
4. **False positive** — Test passes for the wrong reason (e.g., error swallowed)

### What Counts as "Hygiene Violation" (Critical — blocks verification)

1. **PROTOCOL ERROR** — Wrong byte sequence, wrong parameter encoding, wrong behavior for any terminal protocol (CSI, OSC, DCS, Kitty, SGR, mouse, etc.). **Highest severity** — worse than any other violation because it causes silent interop failures
2. **LEAK** — Side logic: rendering in grid code, PTY logic in UI code, duplicated dispatch tables
3. **DRIFT** — Registration data present in one sync point but missing from another
4. **GAP** — Feature supported in one layer but blocked/missing in another (e.g., core supports feature but UI doesn't expose it)
5. **CLAUDE.md violation** — `unwrap()` in library code, `unsafe` blocks, `println!` debugging, function >50 lines, missing cross-platform `#[cfg]` counterpart
6. **Domain rule violation** — violates rules from the relevant `.claude/rules/*.md` file (e.g., `crate-boundaries.md` upward dependency, `test-organization.md` inline tests)

### What Counts as "Hygiene Note" (Informational — does not block verification)

1. **BLOAT** — File exceeds 500-line limit (excluding tests)
2. **WASTE** — Unnecessary `.clone()`, allocation, or transformation at boundary
3. **EXPOSURE** — Internal state leaking through boundary types
4. **Style nit** — Function >30 lines (target, not hard limit), minor naming inconsistency

### What Counts as "Cannot Verify"

1. **No tests exist** — Feature claimed complete but no test coverage
2. **Tests don't cover claim** — Tests exist but don't test the specific feature

### Annotation Requirements

**Be specific.** Every annotation must say exactly what's wrong and what's needed. Every `- [ ]` sub-item must be a concrete, actionable test description that someone can implement without further research.

Good:
```markdown
- INCOMPLETE COVERAGE: grid.scroll_up() — 4/16 cells covered
  - [ ] Add test: scroll_up with CJK characters (wide chars at boundary)
  - [ ] Add test: scroll_up with active selection (selection should adjust)
  - [ ] Add test: scroll_up in alternate screen (no scrollback)
  - [ ] Add test: scroll_up at top of scroll region (region boundary)
  - [ ] Add test: scroll_up with display_offset (user scrolled back)
- NEEDS PIN:
  - [ ] Add semantic pin: scroll_up(1) moves row[0] to scrollback, row[1] becomes row[0]
```

Bad:
```markdown
- NEEDS TESTS: Add more tests
- INCOMPLETE COVERAGE: needs more scenarios
```

---

## Important Constraints

### DO NOT:
- Fix bugs encountered during verification
- Implement missing features
- Modify test files
- Change any code outside `plans/roadmap/`

### DO:
- Run existing tests (always with `timeout 150`)
- Read reference repos for expected behavior
- Annotate items with specific test requirements
- Update checkbox status based on verification
- Track what needs attention

### If You Find a Bug:

In the review results, note it:
```markdown
- [ ] **Implement**: Feature X
  - BUG FOUND: [brief description]
  - Should be fixed before marking complete
```

Additionally, invoke `/add-bug` to file it in the bug tracker. Do NOT fix it — just document and file.

---

## Progress Tracking

### During Session

Supervisor maintains batch-level tracking:
```
Batch 1: [COMPLETE] Sections 1, 2, 3 — committed
Batch 2: [RUNNING]  Sections 4, 5, 5b
  - Section 4 agent: 15/25 items processed
  - Section 5 agent: 8/12 items processed
  - Section 5b agent: done, waiting for batch
Batch 3: [PENDING]  Sections 5c, 6, 7
```

### Between Sessions

If verification is interrupted, the last batch commit shows progress. Resume using:
```
/verify-roadmap continue
```

This resumes from the first unverified section (based on frontmatter status).

Or specify where to start:
```
/verify-roadmap section-3
```

---

## Output Format

### Phase 1 Agent Output (in results file)

Each review agent writes its results in this format — per item, then gap analysis:
```
--- Verifying 1.1: Cell struct ---
Context loaded: CLAUDE.md (read), rules/*.md (4 files read), reference: alacritty/src/term/cell.rs
Tests found: oriterm_core/src/cell/tests.rs (8 tests)
Tests run: all pass
Audit: READ oriterm_core/src/cell/tests.rs
  - line 12: `assert_eq!(cell.c, 'A')` — correct, default char
  - line 18: `assert!(cell.flags.contains(CellFlags::BOLD))` — correct
  - line 25: `assert_eq!(cell.fg, Color::Named(NamedColor::White))` — correct default
Coverage assessment:
  Inputs tested: ASCII, basic Unicode (2/4 — missing CJK, combining marks)
  States tested: default, with flags, with colors (3/3)
  Coverage: 6/12 cells
Semantic pin: line 25 — asserts specific default color, would fail if default changed
Hygiene audit:
  Files checked: oriterm_core/src/cell/mod.rs (180 lines)
  impl-hygiene.md: no violations
  CLAUDE.md: no violations
  crate-boundaries.md: no violations
Status: VERIFIED (sound, coverage 6/12 but relevant cells covered, pin: default_color, hygiene: clean)

--- Verifying 2.3: SGR attribute dispatch ---
Context loaded: CLAUDE.md (read), rules/*.md (4 files read), reference: alacritty, wezterm, ghostty
Tests found: oriterm_core/src/term_handler/tests.rs (12 SGR tests)
Tests run: all pass
Audit: READ oriterm_core/src/term_handler/tests.rs
  - line 45: `handler.input(b"\x1b[1m"); assert!(cell.flags.contains(BOLD))` — correct
  - line 52: `handler.input(b"\x1b[38;2;255;0;0m"); assert_eq!(cell.fg, Rgb(255,0,0))` — correct
  - line 68: `handler.input(b"\x1b[58;2;0;255;0m")` — PROTOCOL ERROR: uses semicolons
Protocol verification:
  Sequence: SGR 58 (underline color)
  Our code: parses `\x1b[58;2;R;G;Bm` (semicolon-separated)
  xterm ctlseqs: SGR 58 uses COLON subparams: `\x1b[58:2::R:G:Bm`
  Reference 1: alacritty_terminal/src/term/color.rs:148 — uses colon subparams
  Reference 2: wezterm/term/src/terminalstate/performer.rs:892 — uses colon subparams
  Reference 3: kitty/terminfo.py — uses colon subparams
  VERDICT: PROTOCOL ERROR — semicolons instead of colons for SGR 58 subparams
  Note: SGR 38/48 accept BOTH semicolons (legacy) and colons (standard),
        but SGR 58 was defined after the colon convention was established
Coverage assessment:
  SGR tested: 1,2,3,4,7,9,22,23,24,27,29,38,48 (13/20 — missing 5,6,8,53,58,59)
  Coverage: 13/20 cells
Semantic pin: line 52 — asserts exact RGB values for truecolor, would fail with wrong param order
Protocol pin: line 45 — CSI 1 m enables bold specifically, not just "some attribute"
  NEEDS PROTOCOL PIN: SGR 58 underline color — no byte-level pin for colon subparam parsing
Hygiene audit:
  Files checked: oriterm_core/src/term_handler/mod.rs (320 lines)
  No violations
Status: NOT VERIFIED — PROTOCOL ERROR (SGR 58 colon subparams), INCOMPLETE COVERAGE (13/20)

--- ... more items ... ---

=== Gap Analysis: Section 1 — Cell + Grid ===
Section goal: "Rich cell type and grid data structure with scrollback"
Listed items cover: cell struct, cell flags, grid storage, cursor, scrollback
GAPS FOUND:
  MISSING FUNCTIONALITY:
  - [ ] **Implement**: Extended cell storage for wide/combined chars — alacritty has this
  - [ ] **Implement**: Row recycling for bounded scrollback — performance invariant requires it
  MISSING TESTS:
  - [ ] **Rust Tests**: oriterm_core/src/grid/tests.rs — reflow with CJK at wrap boundary
  - [ ] **Rust Tests**: oriterm_core/src/grid/tests.rs — scrollback eviction when at max
  MISSING ITEMS:
  - [ ] Wire grid resize to selection coordinate update — both exist but integration untested
  #[ignore] markers found: 1 in oriterm_core/src/grid/tests.rs referencing untracked issue
  TODOs found: 1 in oriterm_core/src/grid/mod.rs:89 — "TODO: handle image cells in reflow"
Items to add: 2 functionality, 2 test, 1 integration
```

**Critical**: Agents MUST show evidence of (1) reading CLAUDE.md + rules, (2) reading test files, (3) coverage assessment, (4) protocol accuracy verification with reference repo citations (for protocol-touching items), (5) pin identification (including byte-level protocol pins), (6) hygiene audit of implementation files, (7) gap analysis. A result like this is REJECTED by the supervisor:
```
--- Verifying 1.1: Cell struct ---
Tests found: oriterm_core/src/cell/tests.rs
Tests run: pass
Status: VERIFIED
```
(No context-loading evidence, no audit, no coverage, no pin, no hygiene audit, no gap analysis — supervisor will flag this agent and re-verify.)

### Final Summary (after Phase 2)
```
=== Verification Complete (Sections 1, 2, 3) ===
Phase 1: All sections reviewed | Phase 2: All section files updated

Section 1 — Cell + Grid:
  Verified:           18/25
  Weak tests:          2
  Incomplete coverage: 3
  Needs pin:           1
  Needs tests:         4
  Regressions:         0
  Reopened:            2
  New items (gaps):    5  <-- added from gap analysis

Section 2 — Terminal State Machine + VTE:
  Verified:              26/38
  Protocol errors:         3  <-- HIGHEST SEVERITY: wrong bytes/params/behavior
  Protocol coverage gaps:  2  <-- protocol test matrices with untested cells
  Weak tests:              1
  Incomplete coverage:     2
  Needs pin:               1
  Wrong tests:             1
  Needs tests:             3
  Regressions:             0
  Hygiene violations:      2  <-- critical: LEAK/DRIFT/GAP/rules (blocks verification)
  Hygiene notes:           3  <-- informational: BLOAT/WASTE/EXPOSURE (does not block)
  Reopened:                0
  New items (gaps):        3  <-- added from gap analysis

  PROTOCOL ERRORS (fix first — highest severity):
  - 2.3: SGR 58 underline color — uses semicolons, spec requires colons
    Refs: alacritty color.rs:148, wezterm performer.rs:892, kitty terminfo.py
  - 2.5: OSC 4 response — returns 16-bit components, should match query bit depth
    Refs: xterm ctlseqs, wezterm osc_response(), alacritty osc.rs:205
  - 2.9: Kitty keyboard — modifier encoding off by 1 (uses modifiers, spec says modifiers+1)
    Refs: kitty keys.py:encode_key_event(), ghostty keyboard.zig:342

  Other items needing attention:
  - 2.3: CSI dispatch — INCOMPLETE COVERAGE (only basic CSI tested, 4/12 cells)
  - 2.7: VTE handler — HYGIENE VIOLATION: LEAK (rendering logic in handler)
  - 2.8: Mode setting — RULES VIOLATION: CLAUDE.md (unwrap in library code)
  - NEW: DCS passthrough — MISSING FUNCTIONALITY (wezterm/ghostty both handle this)
  - NEW: DECRQM mode query — MISSING FUNCTIONALITY (xterm standard)

Section 3 — Cross-Platform:
  Verified:      12/15
  Needs tests:    2
  Needs pin:      1
  New items (gaps): 4
```

---

## Files Modified

**Phase 1 creates (temp):**
- `plans/roadmap/.verify-results/section-XX-results.md` — Review agent findings (can be deleted after Phase 2)

**Phase 2 modifies (the deliverable):**
- `plans/roadmap/section-*.md` — Status updates, annotations, reopened items, AND new items from gap analysis

**Never modifies:**
- Any code files
- Any test files
- Anything outside `plans/roadmap/` (except bug tracker via `/add-bug`)
