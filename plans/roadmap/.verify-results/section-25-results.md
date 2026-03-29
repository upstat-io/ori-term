# Section 25: Theme System -- Verification Results

**Verified:** 2026-03-29
**Section status:** in-progress
**Reviewed gate:** false

---

## Test Execution

```
cargo test -p oriterm scheme
66 passed, 0 failed, 0 ignored (filtered from 2084 total)
cargo test -p oriterm platform::theme
39 passed, 0 failed, 0 ignored
```

All 105 tests pass. No hangs, no flaky tests.

---

## Test Coverage Assessment

### 25.1 Theme Format & Loading (status: complete)

**Test file:** `oriterm/src/scheme/loader/tests.rs` (278 lines, 16 tests)
**Test file:** `oriterm/src/scheme/tests.rs` (413 lines, 32 tests)

Claimed tests vs actual:

| Plan claim | Actual test | Verdict |
|---|---|---|
| Parse valid TOML theme file to ColorScheme | `parse_valid_theme` | PRESENT |
| Reject malformed hex colors with descriptive error | `malformed_hex_rejected`, `parse_theme_3digit_hex_rejected`, `parse_theme_0x_prefix_rejected` | PRESENT (3 tests) |
| Case-insensitive name lookup finds built-in themes | `find_builtin_case_insensitive` | PRESENT |
| User theme overrides built-in theme with same name | `discover_all_user_overrides_builtin` | PRESENT |
| Absolute path loading works for custom theme file | `resolve_scheme_absolute_path` | PRESENT |
| Missing theme file returns error, does not crash | `resolve_scheme_absolute_path_missing_file`, `resolve_scheme_returns_none_for_unknown` | PRESENT |

Additional tests beyond plan: `parse_theme_name_fallback_to_filename`, `wrong_ansi_array_length_rejected`, `extra_ansi_array_length_takes_first_16`, `unknown_fields_ignored`, `discover_themes_finds_valid_files`, `discover_themes_nonexistent_dir`, `discover_themes_empty_dir`, `parse_theme_with_bom`, `parse_theme_with_selection_colors`, `invalid_toml_returns_none`, `missing_required_field_returns_none`.

Coverage verdict: **EXCEEDS plan**. All 6 planned tests present plus 11 additional edge case tests.

### 25.2 Built-in Theme Library (status: complete)

**Source file:** `oriterm/src/scheme/builtin.rs` (689 lines, 54 const BuiltinScheme definitions)

| Plan claim | Actual test | Verdict |
|---|---|---|
| All built-in schemes have valid RGB values | `builtin_schemes_have_valid_rgb`, `all_builtins_produce_valid_palettes` | PRESENT |
| All built-in schemes have unique names | `builtin_names_unique` | PRESENT |
| BUILTIN_SCHEMES array contains 50+ schemes | `builtin_names_not_empty` (asserts >= 50) | PRESENT |
| find_builtin returns correct scheme for each name | `find_builtin_case_insensitive`, `find_builtin_exact_match` | PRESENT |

Plan says 53 schemes; actual count is 54. Exceeds the 50+ target.

Coverage verdict: **MEETS plan**.

### 25.3 Light/Dark Auto-Switch (status: in-progress)

**Test file:** `oriterm/src/scheme/tests.rs` and `oriterm/src/platform/theme/tests.rs`

| Plan claim | Actual test | Verdict |
|---|---|---|
| Parse "dark:X, light:Y" config syntax correctly | `parse_conditional_dark_light` | PRESENT |
| Parse plain "X" config syntax as static theme | `parse_conditional_plain_name` | PRESENT |
| Reversed order "light:Y, dark:X" parses correctly | `parse_conditional_reversed_order` | PRESENT |
| Extra whitespace handled | `parse_conditional_extra_whitespace` | PRESENT |
| Single prefix returns None | `parse_conditional_only_dark` | PRESENT |

Additional: `parse_conditional_duplicate_dark`, `resolve_scheme_name_*` family (7 tests), `config_foreground_overrides_scheme`, `discover_all_*` family (3 tests).

Platform theme detection: 39 tests covering dbus, gsettings, KDE kdeglobals, GTK theme name, desktop environment classification. All pass.

**Incomplete items per plan:** Settings dropdown improvements (blocked by section 7). This is correctly documented in the plan.

Coverage verdict: **EXCEEDS plan** for implemented parts.

---

## Hygiene Audit

### File Size (500-line limit)

| File | Lines | Verdict |
|---|---|---|
| `oriterm/src/scheme/mod.rs` | 179 | OK |
| `oriterm/src/scheme/loader/mod.rs` | 177 | OK |
| `oriterm/src/scheme/builtin.rs` | 689 | **VIOLATION** |

`builtin.rs` is 689 lines, exceeding the 500-line limit. However, this file is pure `const` data (54 color scheme definitions) with zero logic beyond two helper `const fn` (lines 1-43). The code hygiene rule says "Source files (excluding `tests.rs`) must not exceed 500 lines." This is technically a violation. A split would be natural at ~half the schemes (e.g., `builtin_a_m.rs` and `builtin_n_z.rs`), but the file has no logic to extract -- it is a lookup table.

### Test Organization

All test files follow the sibling `tests.rs` pattern correctly:
- `scheme/mod.rs` ends with `#[cfg(test)] mod tests;`
- `scheme/loader/mod.rs` ends with `#[cfg(test)] mod tests;`
- Test files use `super::` imports, no `mod tests { }` wrapper

### Code Hygiene

- Module docs present on all source files (`//!` comments)
- `///` doc comments on all pub items
- No `unwrap()` in library code (loader uses `.ok()?` and `match`)
- No dead code, no commented-out code
- Import organization follows 3-group convention
- `discover_all()` is `#[cfg(test)]` only -- good, avoids dead code in prod

### Impl Hygiene

- One-way data flow: `resolve_scheme_name()` is a pure function, no side effects
- `palette_from_scheme()` bridges scheme to palette cleanly
- `parse_conditional()` is well-separated from resolution
- No circular imports detected

---

## Gap Analysis

### What is complete and working:
1. TOML theme format with full validation (hex parsing, array length, missing fields)
2. 54 built-in schemes (exceeds 50+ target by 4)
3. Theme discovery from config_dir/themes/*.toml
4. Case-insensitive lookup
5. User theme override of built-in names
6. Absolute path loading
7. Conditional dark/light syntax parsing and resolution
8. Platform dark/light mode detection (Linux: dbus, gsettings, KDE, GTK; macOS/Windows stubs)
9. Palette integration (scheme to palette bridge with selection colors)
10. Config override of individual colors (foreground over scheme)
11. Hot-reload support via ConfigMonitor
12. Conversion scripts (iTerm2, Ghostty, base16)

### What is incomplete:
1. **Settings dropdown improvements** (25.3): Group themes by light/dark/universal, show labels. Blocked by Section 7 (settings panel). Correctly marked as blocked.
2. **Section 25.4 completion**: Blocked by the same settings dropdown items.

### Issues found:
1. **`builtin.rs` exceeds 500 lines** (689 lines). Pure const data, but technically violates the rule. Low severity since it is a data file with no logic.

---

## Summary

| Subsection | Status | Tests | Verdict |
|---|---|---|---|
| 25.1 Theme Format & Loading | complete | 16 loader + 32 scheme = 48 | Accurate |
| 25.2 Built-in Theme Library | complete | 4 dedicated + 54 schemes validated | Accurate |
| 25.3 Light/Dark Auto-Switch | in-progress | 12 conditional + 39 platform = 51 | Accurate (blocked items documented) |
| 25.4 Section Completion | in-progress | N/A | Accurate (blocked by Section 7) |

**Overall:** Section is accurately tracked. All checked boxes have corresponding passing tests. The blocked items are properly documented with `<!-- blocked-by:7 -->` comments. One minor hygiene note on `builtin.rs` file length (689 lines of const data).
