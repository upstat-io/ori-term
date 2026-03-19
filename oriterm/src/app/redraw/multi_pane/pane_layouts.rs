//! Pane layout computation for multi-pane tabs.

use crate::session::{DividerLayout, LayoutDescriptor, PaneLayout, Rect, compute_all};

use super::super::App;

impl App {
    /// Compute pane layouts for the active tab.
    ///
    /// Returns `None` if the tab has a single pane (use the fast path).
    /// Returns `Some((pane_layouts, divider_layouts))` when multi-pane.
    pub(in crate::app) fn compute_pane_layouts(
        &self,
    ) -> Option<(Vec<PaneLayout>, Vec<DividerLayout>)> {
        let win_id = self.active_window?;
        let win = self.session.get_window(win_id)?;
        let tab_id = win.active_tab()?;
        let tab = self.session.get_tab(tab_id)?;

        let is_zoomed = tab.zoomed_pane().is_some();

        if !is_zoomed && tab.tree().pane_count() <= 1 && tab.floating().is_empty() {
            return None;
        }

        let ctx = self.focused_ctx()?;
        let bounds = ctx.terminal_grid.bounds()?;
        let cell = ctx.renderer.as_ref()?.cell_metrics();

        // Zoomed: single pane fills the entire available area.
        if let Some(zoomed_id) = tab.zoomed_pane() {
            let avail = Rect {
                x: bounds.x(),
                y: bounds.y(),
                width: bounds.width(),
                height: bounds.height(),
            };
            let cols = (avail.width / cell.width).floor() as u16;
            let rows = (avail.height / cell.height).floor() as u16;
            let snapped_w = cols as f32 * cell.width;
            let snapped_h = rows as f32 * cell.height;
            return Some((
                vec![PaneLayout {
                    pane_id: zoomed_id,
                    pixel_rect: Rect {
                        x: avail.x,
                        y: avail.y,
                        width: snapped_w,
                        height: snapped_h,
                    },
                    cols: cols.max(1),
                    rows: rows.max(1),
                    is_focused: true,
                    is_floating: false,
                }],
                vec![],
            ));
        }

        let desc = LayoutDescriptor {
            available: Rect {
                x: bounds.x(),
                y: bounds.y(),
                width: bounds.width(),
                height: bounds.height(),
            },
            cell_width: cell.width,
            cell_height: cell.height,
            divider_px: self.config.pane.divider_px,
            min_pane_cells: self.config.pane.min_cells,
        };

        let (panes, dividers) = compute_all(tab.tree(), tab.floating(), tab.active_pane(), &desc);
        Some((panes, dividers))
    }
}
