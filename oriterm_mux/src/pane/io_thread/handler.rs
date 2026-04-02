//! Command handler methods for [`PaneIoThread`].
//!
//! Extracted from `mod.rs` to keep file sizes under the 500-line limit.
//! Contains `handle_command()`, `handle_reply_command()`, `process_resize()`,
//! and `build_zone_selection()`.

use std::sync::atomic::Ordering;

use oriterm_core::{EventListener, Selection, SelectionMode, SelectionPoint, Term};

use super::{PaneIoCommand, PaneIoThread};

impl<T: EventListener> PaneIoThread<T> {
    /// Process a resize command on the IO thread.
    ///
    /// Performs grid reflow, then sends SIGWINCH to the PTY. The ordering
    /// is critical: reflow first so the shell sees the correct dimensions
    /// when it handles SIGWINCH. Uses `Term::resize()` (not
    /// `Grid::resize()`) to also resize the alt grid and prune image caches.
    pub(super) fn process_resize(&mut self, rows: u16, cols: u16) {
        self.terminal.resize(rows as usize, cols as usize, true);
        self.grid_dirty.store(true, Ordering::Release);

        // PTY resize with dedup — skip syscall if dimensions unchanged.
        let packed = (rows as u32) << 16 | cols as u32;
        if self.last_pty_size != packed {
            self.last_pty_size = packed;
            if let Some(ref ctl) = self.pty_control {
                if let Err(e) = ctl.resize(rows, cols) {
                    log::warn!("PTY resize failed: {e}");
                }
            }
        }
    }

    /// Build a line selection from a range-finding function on the terminal.
    ///
    /// Used by `SelectCommandOutput` and `SelectCommandInput` commands.
    fn build_zone_selection(
        &self,
        range_fn: impl FnOnce(&Term<T>, usize) -> Option<(usize, usize)>,
    ) -> Option<Selection> {
        let grid = self.terminal.grid();
        let sb_len = grid.scrollback().len();
        let viewport_center = sb_len.saturating_sub(grid.display_offset()) + grid.lines() / 2;
        let (start_row, end_row) = range_fn(&self.terminal, viewport_center)?;
        let start_stable = oriterm_core::grid::StableRowIndex::from_absolute(grid, start_row);
        let end_stable = oriterm_core::grid::StableRowIndex::from_absolute(grid, end_row);
        let anchor = SelectionPoint {
            row: start_stable,
            col: 0,
            side: oriterm_core::index::Side::Left,
        };
        let pivot = SelectionPoint {
            row: end_stable,
            col: usize::MAX,
            side: oriterm_core::index::Side::Right,
        };
        Some(Selection {
            mode: SelectionMode::Line,
            anchor,
            pivot,
            end: anchor,
        })
    }

    /// Handle a command from the main thread.
    ///
    /// Display-affecting commands (scroll, theme, cursor, dirty) are handled
    /// here so the IO thread's `Term` stays in sync with user operations.
    /// Resize is handled separately via `process_resize()` with coalescing.
    pub(super) fn handle_command(&mut self, cmd: PaneIoCommand) {
        log::trace!("IO thread: command {cmd:?}");
        match cmd {
            PaneIoCommand::Resize { rows, cols } => self.process_resize(rows, cols),
            PaneIoCommand::ScrollDisplay(delta) => {
                self.terminal.grid_mut().scroll_display(delta);
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::ScrollToBottom => {
                self.terminal.grid_mut().scroll_display(isize::MIN);
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SetTheme(theme, palette) => {
                self.terminal.set_theme(theme);
                *self.terminal.palette_mut() = *palette;
                self.terminal.grid_mut().dirty_mut().mark_all();
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SetCursorShape(shape) => {
                self.terminal.set_cursor_shape(shape);
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SetBoldIsBright(enabled) => {
                self.terminal.set_bold_is_bright(enabled);
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::MarkAllDirty => {
                self.terminal.grid_mut().dirty_mut().mark_all();
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SetImageConfig(config) => {
                self.terminal.set_image_protocol_enabled(config.enabled);
                self.terminal
                    .set_image_limits(config.memory_limit, config.max_single);
                self.terminal
                    .set_image_animation_enabled(config.animation_enabled);
            }
            PaneIoCommand::ScrollToPreviousPrompt => {
                self.terminal.scroll_to_previous_prompt();
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::ScrollToNextPrompt => {
                self.terminal.scroll_to_next_prompt();
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::OpenSearch => {
                self.search = Some(oriterm_core::SearchState::new());
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::CloseSearch => {
                self.search = None;
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SearchSetQuery(query) => {
                if let Some(ref mut s) = self.search {
                    s.set_query(query, self.terminal.grid());
                    self.grid_dirty.store(true, Ordering::Release);
                }
            }
            PaneIoCommand::SearchNextMatch => {
                if let Some(ref mut s) = self.search {
                    s.next_match();
                    self.grid_dirty.store(true, Ordering::Release);
                }
            }
            PaneIoCommand::SearchPrevMatch => {
                if let Some(ref mut s) = self.search {
                    s.prev_match();
                    self.grid_dirty.store(true, Ordering::Release);
                }
            }
            PaneIoCommand::Reset => {} // No Term::reset() exists yet.
            PaneIoCommand::Shutdown => {
                self.shutdown.store(true, Ordering::Release);
            }
            // Request-response commands with reply channels.
            other => self.handle_reply_command(other),
        }
    }

    /// Handle request-response commands that use a reply channel.
    pub(crate) fn handle_reply_command(&mut self, cmd: PaneIoCommand) {
        match cmd {
            PaneIoCommand::ExtractText { selection, reply } => {
                let text = oriterm_core::selection::extract_text(self.terminal.grid(), &selection);
                let result = if text.is_empty() { None } else { Some(text) };
                let _ = reply.send(result);
            }
            PaneIoCommand::ExtractHtml {
                selection,
                font_family,
                font_size,
                reply,
            } => {
                let (html, text) = oriterm_core::selection::extract_html_with_text(
                    self.terminal.grid(),
                    &selection,
                    self.terminal.palette(),
                    &font_family,
                    font_size,
                );
                let result = if text.is_empty() {
                    None
                } else {
                    Some((html, text))
                };
                let _ = reply.send(result);
            }
            PaneIoCommand::EnterMarkMode { reply } => {
                if self.terminal.grid().display_offset() > 0 {
                    self.terminal.grid_mut().scroll_display(isize::MIN);
                }
                let g = self.terminal.grid();
                let cursor = g.cursor();
                let abs_row = g.scrollback().len() + cursor.line();
                let stable = oriterm_core::grid::StableRowIndex::from_absolute(g, abs_row);
                let mc = crate::pane::MarkCursor {
                    row: stable,
                    col: cursor.col().0,
                };
                let _ = reply.send(mc);
                self.grid_dirty.store(true, Ordering::Release);
            }
            PaneIoCommand::SelectCommandOutput { reply } => {
                let sel = self.build_zone_selection(Term::command_output_range);
                let _ = reply.send(sel);
            }
            PaneIoCommand::SelectCommandInput { reply } => {
                let sel = self.build_zone_selection(Term::command_input_range);
                let _ = reply.send(sel);
            }
            _ => {} // All other variants handled in handle_command.
        }
    }
}
