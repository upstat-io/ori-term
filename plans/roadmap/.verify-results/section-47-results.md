# Section 47 Verification: Semantic Prompt State Management

## Status: NOT STARTED (confirmed)

### Evidence Search

**No `SemanticContentType`, `SemanticType`, `RowPromptFlag`, or `semantic_range` types exist** anywhere in the codebase. The plan's proposed types are entirely new.

**However, substantial existing infrastructure exists** that this section builds upon and replaces:

1. **`PromptState` enum** (`oriterm_core/src/term/mod.rs`, line 40): Already exists with variants `None`, `PromptStart`, `CommandStart`, `OutputStart`. This is the state machine that tracks which OSC 133 phase the terminal is in. Section 47 does NOT replace this; it adds cell/row-level flags driven BY this state machine.

2. **`PromptMarker` struct** (`oriterm_core/src/term/mod.rs`, line 58): The existing marker-based approach that Section 47 plans to replace. Fields: `prompt: usize` (absolute row), `command: Option<usize>`, `output: Option<usize>`.

3. **`prompt_markers: Vec<PromptMarker>`** (`oriterm_core/src/term/mod.rs`, line 162): The existing storage. Section 47.8 explicitly plans to remove this.

4. **`shell_state.rs`** (`oriterm_core/src/term/shell_state.rs`): 353 lines of prompt navigation, marker management, pruning. Contains `scroll_to_previous_prompt()`, `scroll_to_next_prompt()`, `command_output_range()`, `command_input_range()`, `prune_prompt_markers()`, `mark_prompt_row()`, `mark_command_start_row()`, `mark_output_start_row()`. All of these are the consumers that Section 47.8 plans to migrate.

5. **OSC 133 interception** (`oriterm_mux/src/shell_integration/interceptor.rs`): Intercepts OSC 133 A/B/C/D and sets `prompt_state` on the `Term` struct. Tests verify all four transitions.

6. **Shell integration scripts** (`oriterm_mux/shell-integration/`): bash, zsh, fish, powershell scripts all emit OSC 133;A/B/C/D sequences. Well-tested.

7. **Pending marks system** (`PendingMarks` bitflags in `oriterm_core/src/term/mod.rs`): Deferred marking mechanism so row marking happens after both VTE parsers finish. This is an important implementation detail that Section 47 should preserve.

### CellFlags Capacity

`CellFlags` is currently `u16` with bits 0-15 used. All 16 bits are assigned:
- Bits 0-7: SGR attributes (bold, dim, italic, underline, blink, inverse, hidden, strikethrough)
- Bits 8-10: Wide char/spacer/wrap
- Bits 11-14: Underline variants (curly, dotted, dashed, double)
- Bit 15: Leading wide char spacer

**CRITICAL FINDING**: The plan says "use 2 bits from the existing bitflags (or add 2 bits)" but all 16 bits of the `u16` are occupied. Adding 2 bits for `SemanticContentType` requires either:
- Expanding `CellFlags` to `u32` (doubles flags size, potentially increases Cell size beyond the 24-byte target -- currently asserted at compile time: `assert!(size_of::<Cell>() <= 24)`)
- Using a separate `u8` field on `Cell` (also increases Cell size)
- Packing into the 2-byte padding slot already present in Cell layout (`char(4) + Color(4) + Color(4) + CellFlags(2) + pad(2) + Option<Arc>(8)` = 24 bytes)

The padding slot between `CellFlags(2)` and `Option<Arc>(8)` is exactly 2 bytes. A separate `u8` field for semantic type (or a combined `u8` with row flags) could fit there without increasing Cell size. But the plan doesn't address this constraint.

### Row Struct Analysis

The `Row` struct (`oriterm_core/src/grid/row/mod.rs`) currently has only two fields: `inner: Vec<Cell>` and `occ: usize`. Adding a 2-bit `RowPromptFlag` is straightforward -- a `u8` field will fit without alignment issues. The plan is correct here.

### TODOs/FIXMEs

No TODOs or FIXMEs found related to semantic prompts, prompt state, or OSC 133 handling.

### Gap Analysis

1. **CellFlags overflow**: As noted above, the plan claims 2 bits can be packed into existing CellFlags but all 16 bits are used. The plan must explicitly address Cell size constraint. The 2-byte padding between `CellFlags` and `Option<Arc>` is the natural placement, but it requires a new field, not packing into `CellFlags`. This is a plan correctness issue.

2. **Pending marks system**: The plan doesn't mention the `PendingMarks` bitflags mechanism. Cell stamping (47.8) happens during `put_char`/`input()`, but row marking must still be deferred. The existing `pending_marks` pattern needs to be preserved or adapted, not silently dropped.

3. **Prune on eviction**: The plan mentions removing `Vec<PromptMarker>` but doesn't address what replaces `prune_prompt_markers()`. With row/cell flags, eviction of scrollback rows naturally evicts their flags -- no separate pruning needed. This is actually simpler, but the plan should call it out as a simplification.

4. **Resize prompt clearing (47.5)**: The plan references the Kitty `redraw=0` extension but the current shell integration scripts (`oriterm_mux/shell-integration/`) do not emit `k=s`, `k=c`, or `redraw=0` parameters with OSC 133;A. The VTE parser and interceptor would need to parse these parameters. This is not mentioned in the plan.

5. **Click-to-move (47.7)**: References `oriterm/src/app/mouse_input.rs` which exists and handles mouse events. The guard conditions are reasonable but the plan should mention that mouse reporting check requires reading `term.mode()` for `TermMode::MOUSE_MODE`.

6. **Good**: The plan correctly identifies the migration path from `Vec<PromptMarker>` to cell/row flags and the need to upgrade all existing consumers. The Ghostty PR reference is appropriate.

7. **Good**: The `has_semantic_prompts` fast-path flag (47.6) is a clean optimization for terminals without shell integration.

### Infrastructure from Other Sections

- **Section 01 (Cell + Grid)**: Complete. Provides the Cell and Grid types that this section modifies.
- **Section 02 (Term + VTE)**: Complete. Provides the VTE handler where OSC 133 is processed.
- **Section 20 (Shell Integration)**: Complete. Provides the current marker-based approach that this section replaces.
- **Section 40 (Vi Mode)**: Not started. Would benefit from row flags for `[[`/`]]` motions.
- **Section 41 (Hints)**: Not started. Would benefit from exact output region detection.

### Verdict

**CONFIRMED NOT STARTED**. No semantic content types or row prompt flags exist. The plan is well-structured but has a **critical gap** around CellFlags capacity -- all 16 bits of the `u16` are occupied. The implementation must either use the existing 2-byte padding in the Cell struct or expand the flags type, and the Cell size assertion (`<= 24 bytes`) must be preserved. The plan should also address VTE parsing of OSC 133 parameters (`k=s`, `k=c`, `redraw=0`) and explicitly note that scrollback eviction naturally handles flag cleanup.
