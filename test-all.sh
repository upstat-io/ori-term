#!/usr/bin/env bash
set -euo pipefail

# Match CI: promote all warnings to errors so local test catches the same
# lint failures that RUSTFLAGS="-D warnings" catches in GitHub Actions.
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings"

echo "=== cargo test --workspace --features oriterm/gpu-tests ==="
cargo test --workspace --features oriterm/gpu-tests

# Section 01.3 catalog coverage gate — fails on schema drift, Phase 2
# Finding J anti-LEAK, Phase 2 Finding L bootstrap verification, and
# line-number-primary Implementation citations.
echo ""
echo "=== catalog_coverage_check --check --bootstrap-mode (real catalog) ==="
if [ -d plans/spec-conformance/catalog ]; then
    cargo run --quiet -p oriterm_test_support --bin catalog_coverage_check -- check --bootstrap-mode

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
else
    echo "SKIP catalog_coverage_check: plans/spec-conformance/catalog not present"
fi

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
