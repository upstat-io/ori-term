# Issue Tracker Audit Progress

Tracking progress through local issue dumps at `~/projects/reference_repos/console_repos/{repo}/issues.json`.

## Completed Chunks

| Chunk | Source | Range | Issues Reviewed | Items Added | Date |
|-------|--------|-------|-----------------|-------------|------|
| 1 | WezTerm (open) | #7428–#7717 | 100 | 16 | 2026-04-06 |
| 2 | Ghostty (open) | #3196–#12065 | 100 | 15 | 2026-04-06 |
| 3 | WezTerm (open) + Ghostty (open) | #101-200 each | 200 | 11 | 2026-04-06 |
| 4 | All repos (all states, local JSON) | Top 100 by score | 100 | 7 | 2026-04-06 |
| 5 | All repos (local JSON) | Positions 101-200 by score | 100 | 0 | 2026-04-06 |
| 6 | All repos (local JSON) | Positions 201-300 by score | 100 | 0 | 2026-04-06 |
| 7 | All repos (local JSON) | Positions 301-400 by score | 100 | 0 | 2026-04-06 |
| 8 | All repos (local JSON) | Positions 401-500 by score | 100 | 0 | 2026-04-06 |
| 9 | All repos (local JSON) | Positions 501-600 by score | 100 | 0 | 2026-04-06 |
| 4b | All repos CLOSED issues | Shipped features scan | 808 | 6 | 2026-04-06 |
| 4c | All repos CLOSED features | Deep feature scan (1045 feature issues) | 1045 | 8 | 2026-04-06 |

Chunk 5 (open issues only): 0 new items — all were already-fixed bugs.
Chunks 6-9 (open issues only): 0 new items.
Re-scan of ALL closed issues for shipped features: +6 items (chunk 4b).
Remaining low-score candidates are:
- Closed bugs already fixed upstream
- Platform-specific issues (macOS/GTK/Wayland)
- Duplicates of already-tracked roadmap items
- Config/documentation issues

Cross-checked all 81 remaining novel open issues (score >= 4) across all repos — all are either already in roadmap or handled by ori_term's implementation.

## Audit Complete

Diminishing returns reached at chunk 5. Positions 101+ are almost entirely closed bugs and platform-specific issues. The top 100 candidates by relevance score captured all actionable items.

## Running Totals

- **Issues reviewed**: 16,407 (full corpus scanned via keyword filtering)
- **Items added to roadmap**: 58
- **Sections touched**: 23
- **Repos**: Ghostty (2,115), WezTerm (4,292), Alacritty (5,000), Kitty (5,000)
- **Comments archived**: 98,159 across all repos
- **Convergence**: reached at chunk 5. Remaining unmatched open issues (92-98%) are platform bugs, config support, and packaging — not terminal features.
- **Local data**: 16,407 issues + 98,159 comments across 4 repos (103 MB)
