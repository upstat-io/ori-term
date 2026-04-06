# Issue Tracker Audit Progress

Tracking progress through local issue dumps at `~/projects/reference_repos/console_repos/{repo}/issues.json`.

## Completed Chunks

| Chunk | Source | Range | Issues Reviewed | Items Added | Date |
|-------|--------|-------|-----------------|-------------|------|
| 1 | WezTerm (open) | #7428–#7717 | 100 | 16 | 2026-04-06 |
| 2 | Ghostty (open) | #3196–#12065 | 100 | 15 | 2026-04-06 |
| 3 | WezTerm (open) + Ghostty (open) | #101-200 each | 200 | 11 | 2026-04-06 |

## Next Chunk

- **Chunk 4**: WezTerm all-states, sorted by number desc, offset 0
- **Chunk 5**: Ghostty all-states, sorted by number desc, offset 0
- Continue alternating until 1,000 total

## Filtering Rules

Skip: packaging, CI/build, macOS-only UI, GTK-only UI, documentation, duplicates of already-audited open issues.
Focus: terminal emulation, VTE, rendering, font, input encoding, performance, security, cross-platform features.

## Running Totals

- **Issues reviewed**: 400
- **Items added to roadmap**: 42
- **Sections touched**: 20
