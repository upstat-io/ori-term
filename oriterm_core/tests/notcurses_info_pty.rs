//! Real-process notcurses-info smoke test under a Unix PTY.
//!
//! Spawns `notcurses-info` inside a real PTY via `PtySession`. Our
//! `Term<PtyResponder>` parses the queries notcurses-info emits and the
//! responder writes back DA1, kitty-graphics, XTSMGRAPHICS, CSI 14t /
//! 18t replies so that notcurses-info's
//! `notcurses_check_pixel_support()` gate (`tcache.cellpxy != 0 &&
//! tcache.cellpxx != 0`) opens and `display_logo()` runs.
//!
//! See: bug-tracker/plans/BUG-06-073/
//!
//! Phase 1 evidence: if `image_count()` and `placement_count()` both
//! reach at least one after a real `notcurses-info` run, the Linux
//! end-to-end path between our handler and real notcurses-info works.
//! Skipped on Windows (cross-compile target) — `notcurses-info` is a
//! Linux binary.

#![cfg(unix)]

use oriterm_core::RenderableContent;
use oriterm_test_support::{PtySession, notcurses_info_available, tool_available};
use portable_pty::CommandBuilder;

/// notcurses-info exits cleanly after dumping its capability info to
/// stdout — our session can wait for child exit and then inspect the
/// terminal's image cache.
///
/// **What this test pins.** With our responder answering every probe
/// in the captured-handshake fixture (`notcurses_info_handshake.rs`
/// already covers the reply-byte presence), real `notcurses-info`
/// chooses a pixel blitter and emits image-protocol bytes that our
/// handler stores. The image cache going from 0 to ≥1 is the
/// end-to-end signal that the protocol round-trip works on Linux.
///
/// **Why it must be Linux-only.** `notcurses-info` is a Linux ELF;
/// running it on Windows requires WSL+bash, which a pure `portable_pty`
/// Windows `ConPTY` `PtySession` cannot host (the WSL bridge is not in
/// the test scope). This test is therefore Phase 1 evidence for the
/// Linux leg of the protocol chain; the Windows symptom narrowing in
/// §01 stays the open question.
#[test]
fn notcurses_info_real_pty_emits_pixel_protocol() {
    if !notcurses_info_available() {
        eprintln!("SKIP: notcurses-info not installed");
        return;
    }
    if !tool_available("infocmp", "-V") {
        eprintln!("SKIP: ncurses tooling (infocmp) not available");
        return;
    }

    let mut cmd = CommandBuilder::new("notcurses-info");
    cmd.env("TERM", "xterm-256color");

    let mut session = PtySession::spawn(cmd, 80, 24);

    // notcurses-info finishes its handshake + display_logo + capability
    // dump in well under a second on a warm machine. 5 s is a generous
    // ceiling for cold-start, GitHub CI runners, and slow disks.
    let status = session.wait_for_child_exit(5_000);
    assert!(
        status.success(),
        "notcurses-info exited unsuccessfully: {status:?}"
    );

    let image_count = session.term().image_cache().image_count();
    let placement_count = session.term().image_cache().placement_count();
    let grid = session.grid_text();
    eprintln!(
        "notcurses_info_pty: image_count={image_count} placement_count={placement_count} \
         exit={status:?}"
    );

    assert!(
        image_count >= 1 && placement_count >= 1,
        "notcurses-info real PTY run did not emit pixel-protocol bytes \
         (image_count={image_count}, placement_count={placement_count}). \
         Phase 1 evidence: either our reply path is missing a probe, \
         OR notcurses skipped display_logo due to a missing/zero \
         cellpx capability. Grid:\n{grid}"
    );
}

/// Verify the full daemon-side snapshot extraction path produces a
/// `RenderableContent` with BOTH placements AND populated `image_data`
/// when notcurses-info has finished rendering.
///
/// **What this test pins (Phase 1 evidence layer 2).** After
/// `notcurses_info_real_pty_emits_pixel_protocol` confirms the handler
/// stored placements, this test exercises `Term::renderable_content_into`
/// (the canonical IO-thread snapshot extraction call site at
/// `oriterm_mux/src/pane/io_thread/mod.rs:481`). The wire path that
/// reaches the GUI starts with this call's output. The two assertions
/// below are the daemon→GUI contract:
///
/// - `content.images` non-empty — placements made it into the snapshot.
/// - `content.image_data` non-empty AND every placement's `image_id`
///   resolves to a `RenderableImageData` entry with non-zero
///   `width × height × data.len()` — pixel bytes are physically present.
///
/// If either fails on Linux, the daemon→GUI snapshot path drops image
/// data BEFORE wire encoding even runs, and the wordmark gap has a
/// platform-independent reproducer. If both pass, the bug is downstream
/// of `renderable_content_into` (wire transport, client decode, or GPU
/// render).
///
/// See: bug-tracker/plans/BUG-06-073/
#[test]
fn notcurses_info_renderable_content_carries_image_data() {
    if !notcurses_info_available() {
        eprintln!("SKIP: notcurses-info not installed");
        return;
    }
    if !tool_available("infocmp", "-V") {
        eprintln!("SKIP: ncurses tooling (infocmp) not available");
        return;
    }

    let mut cmd = CommandBuilder::new("notcurses-info");
    cmd.env("TERM", "xterm-256color");
    let mut session = PtySession::spawn(cmd, 80, 24);
    let status = session.wait_for_child_exit(5_000);
    assert!(status.success(), "notcurses-info exited: {status:?}");

    let mut content = RenderableContent::default();
    session.term().renderable_content_into(&mut content);

    eprintln!(
        "renderable_content_into: images.len={} image_data.len={} images_dirty={}",
        content.images.len(),
        content.image_data.len(),
        content.images_dirty,
    );
    for (i, img) in content.image_data.iter().enumerate() {
        eprintln!(
            "  image_data[{i}]: id={:?} {}x{} px, data.len={}B",
            img.id,
            img.width,
            img.height,
            img.data.len(),
        );
    }
    for (i, p) in content.images.iter().enumerate() {
        eprintln!(
            "  images[{i}]: id={:?} viewport=({},{}) display={}x{} z={}",
            p.image_id, p.viewport_x, p.viewport_y, p.display_width, p.display_height, p.z_index,
        );
    }

    assert!(
        !content.images.is_empty(),
        "snapshot extraction lost the placement: images.len()=0 \
         despite image_cache having {} placements",
        session.term().image_cache().placement_count(),
    );
    assert!(
        !content.image_data.is_empty(),
        "snapshot extraction has placements but no image_data — \
         pixel bytes will not reach the GUI. images.len()={}, \
         image_data.len()=0.",
        content.images.len(),
    );

    let referenced_ids: std::collections::HashSet<_> =
        content.images.iter().map(|p| p.image_id).collect();
    let provided_ids: std::collections::HashSet<_> =
        content.image_data.iter().map(|d| d.id).collect();
    let missing: Vec<_> = referenced_ids.difference(&provided_ids).collect();
    assert!(
        missing.is_empty(),
        "snapshot has placements referencing image ids with no \
         image_data entry — those placements will render blank: \
         referenced={referenced_ids:?} provided={provided_ids:?} missing={missing:?}"
    );

    for img in &content.image_data {
        assert!(
            img.width > 0 && img.height > 0,
            "image_data entry for {:?} has zero dimensions {}x{} — \
             the GPU pipeline will skip a zero-size placement",
            img.id,
            img.width,
            img.height,
        );
        assert!(
            !img.data.is_empty(),
            "image_data entry for {:?} has empty pixel buffer \
             (dims {}x{}) — the wire ships placeholder metadata only",
            img.id,
            img.width,
            img.height,
        );
        let expected_min_bytes = (img.width as usize) * (img.height as usize);
        assert!(
            img.data.len() >= expected_min_bytes,
            "image_data for {:?} carries {}B of pixels but dims \
             {}x{} would need at least {}B (1 byte/pixel even at \
             worst-case grayscale)",
            img.id,
            img.data.len(),
            img.width,
            img.height,
            expected_min_bytes,
        );
    }
}
