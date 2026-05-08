#!/usr/bin/env bash
set -euo pipefail

# Match CI: promote all warnings to errors so local test catches the same
# lint failures that RUSTFLAGS="-D warnings" catches in GitHub Actions.
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings"

# The uncataloged-sequence spool is populated by tests via
# `UncatalogedDetector::serialize_to_dir`. Clear it so
# `spec-coverage-report --check` runs against an empty spool and matches
# CI's "lint-before-tests" ordering (where the rust-cache otherwise
# restores stale spool entries across runs). The OSC TupleSig
# canonicalization SSOT was fixed in BUG-07-019 (commit 49ccb2e0); the
# remaining post-test `--check` BACKLOG categories (charset designation
# Da/Esc, DCS, CSI', C0) are tracked separately and currently keep the
# `--check` gate ordered before tests. Once those clear, the gate can
# move post-test to detect genuine new sequences.
rm -rf target/spec-chain-uncataloged

# prose-lint runs FIRST — fail-fast on authored-doc style drift before
# burning 5-min on tests OR letting the assistant claim "test-all green"
# while a prose-lint violation slips into a /commit-push pre-flight.
# Duplicates the build-all.sh gate intentionally: test-all runs after every
# change per CLAUDE.md §Commands and is the canonical "everything green"
# check; prose-lint must be impossible to bypass via the test-all path.
echo "=== prose-lint (term_repo) ==="
if ! python3 scripts/prose-lint.py .; then
    echo "prose-lint failed — see violations above."
    exit 1
fi

echo ""
# Spec-conformance coverage gates run after prose-lint — same ordering
# CI uses to fail-fast on doc + catalog drift before any cargo work.
echo "=== spec-coverage-report --check (static gates) ==="
cargo run --quiet -p oriterm_test_support --bin spec-coverage-report -- --check

echo ""
echo "=== spec-coverage-report --check audit-files ==="
cargo run --quiet -p oriterm_test_support --bin spec-coverage-report -- --check audit-files

echo ""
echo "=== cargo test --workspace --features oriterm/gpu-tests ==="
cargo test --workspace --features oriterm/gpu-tests

# Section 01.3 catalog coverage gate — fails on schema drift, Phase 2
# Finding J anti-LEAK, and line-number-primary Implementation citations.
#
# Runs in Normal mode (the post-Section-04.7 mode). The original
# `--bootstrap-mode` invocation was a Section 01 gate that forbade any
# `verified` row; it was explicitly bounded to "until Section 04.7
# freezes the schema" (see `CheckMode::Bootstrap` rustdoc and
# `--bootstrap-mode` CLI help). Section 08+ verification work
# legitimately drives rows to `verified`, so the real-catalog check now
# uses Normal mode. The `--bootstrap-mode` API itself is still validated
# via the fixture-injection smoke test below.
echo ""
echo "=== catalog_coverage_check --check (real catalog, Normal mode) ==="
# The binary handles its own graceful-skip when the wrapper repo isn't
# discoverable (post-BUG-08-028 — uses `oriterm_test_support::paths`).
# Don't gate on cwd-relative `plans/spec-conformance/catalog` here — that
# path arithmetic is broken under the wrapper/subrepo layout (plans/ lives
# at the wrapper root, not under term_repo/).
cargo run --quiet -p oriterm_test_support --bin catalog_coverage_check -- check

echo ""
echo "=== sanity check: catalog::tests module is present in the runnable set ==="
# `cargo test --list` writes test names to STDOUT but also writes a
# "Running ..." progress line to STDERR. We merge stderr so that
# tooling emitting progress noise does not starve the pipe.
catalog_tests=$(cargo test -p oriterm_test_support --lib -- --list 2>&1 | grep -c '^catalog::tests::' || true)
if [ "${catalog_tests}" -eq 0 ]; then
    echo "ERROR: catalog::tests module has zero runnable tests — regression in 01.3"
    exit 1
fi
echo "Found ${catalog_tests} catalog::tests::* test(s)."

# Section 01.3 fixture-injection smoke test — proves the --bootstrap-mode
# gate rejects deliberately-bad rows. Uses the committed `.md` fixtures
# under `crates/oriterm_test_support/tests/fixtures/catalog/`, which
# contain:
#   1. `catalog_golden.md`           — well-formed row (should pass)
#   2. `catalog_verified_status.md`  — Phase 2 Finding L violation
#   3. `catalog_wezterm_spec_source.md` — Phase 2 Finding J violation
#   4. `catalog_line_number_primary.md` — banned citation style
#
# Expected: exit code 1 with >= 3 findings. The smoke test flips the
# cargo exit code so a passing gate (exit 0 against bad fixtures) fails
# the script — that would mean the gate silently accepted violations.
echo ""
echo "=== catalog_coverage_check fixture-injection smoke test ==="
if [ -d crates/oriterm_test_support/tests/fixtures/catalog ]; then
    set +e
    cargo run --quiet -p oriterm_test_support --bin catalog_coverage_check -- \
        check --catalog-dir crates/oriterm_test_support/tests/fixtures/catalog --bootstrap-mode \
        > /dev/null 2>&1
    smoke_exit=$?
    set -e
    if [ "${smoke_exit}" -eq 0 ]; then
        echo "ERROR: catalog_coverage_check accepted the deliberately-bad fixture directory (exit 0)"
        echo "       The --bootstrap-mode gate is broken — at least 3 findings should have been raised."
        exit 1
    fi
    echo "Fixture injection smoke test: gate correctly rejected bad fixtures (exit ${smoke_exit})."
else
    echo "SKIP fixture-injection smoke test: fixtures directory not present"
fi

echo ""
echo "All tests passed."
