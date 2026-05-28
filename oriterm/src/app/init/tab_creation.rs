//! Initial-tab and handoff-tab creation helpers used by `App::init`.
//!
//! Extracted from `app/init/mod.rs` to keep that file under the 500-line
//! budget. `create_initial_tab` is the spawn path; `create_handoff_tab`
//! is the Windows console-handoff path (mirrors `create_initial_tab` but adopts
//! pre-existing PTY handles from `conhost.exe`).

use oriterm_mux::domain::SpawnConfig;

use crate::app::App;
use crate::app::config_reload;

/// Grid and cell dimensions for initial-tab creation.
#[derive(Clone, Copy)]
pub(in crate::app) struct InitialTabGeometry {
    pub rows: u16,
    pub cols: u16,
    pub cell_w: u16,
    pub cell_h: u16,
}

impl App {
    /// Create the initial tab from a Windows console handoff payload.
    ///
    /// Section 03.9 Phase 4. Mirrors [`create_initial_tab`] but uses
    /// `EmbeddedMux::adopt_pane` to wrap the pre-existing PTY handles
    /// from `conhost.exe` instead of spawning a fresh shell. The
    /// handoff struct also carries the title (from
    /// `TERMINAL_STARTUP_INFO.pszTitle`) and initial pane dimensions
    /// (from `dwYCountChars` / `dwXCountChars`); we apply both before
    /// the IO thread starts producing snapshots.
    ///
    /// Daemon-mode mux is rejected — the handoff path is exclusively
    /// embedded mode (the COM server is a `REGCLS_SINGLEUSE` process
    /// that handles exactly one console session).
    #[cfg(target_os = "windows")]
    pub(in crate::app) fn create_handoff_tab(
        &mut self,
        session_wid: crate::session::WindowId,
        handoff: crate::platform::default_terminal::handoff::HandoffData,
        cell_w: u16,
        cell_h: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use oriterm_mux::AdoptedPtyHandle;

        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);
        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);

        let crate::platform::default_terminal::handoff::HandoffData {
            reader,
            writer,
            signal,
            client_pid,
            title,
            icon_path,
            initial_rows,
            initial_cols,
        } = handoff;

        let adopted = AdoptedPtyHandle::new(reader, writer, signal, client_pid);

        // The MuxBackend trait's default `adopt_pane` returns "not
        // supported" for daemon mode; only EmbeddedMux overrides it.
        // The handoff path is exclusive to embedded mode because the
        // COM server runs as a `REGCLS_SINGLEUSE` standalone process.
        // The title and icon travel through `AdoptPaneRequest` so
        // `InProcessMux::adopt_standalone_pane` can apply them via
        // `Pane::set_title`/`Pane::set_icon_name` before the pane is
        // registered — `set_title` flips `has_explicit_title` so OSC
        // 0/2 from the shell can later override but the.lnk-derived
        // title is what users see first.
        let title_for_log = title.clone();
        let icon_for_log = icon_path.clone();
        let request = oriterm_mux::backend::AdoptPaneRequest {
            rows: initial_rows,
            cols: initial_cols,
            scrollback: self.config.terminal.scrollback,
            theme,
            initial_title: title,
            initial_icon: icon_path,
        };
        let cfg = &self.config;
        let mux = self.mux.as_mut().ok_or("mux backend missing")?;
        let pane_id = mux.adopt_pane(adopted, request)?;

        // Apply the same per-pane setup the spawn path uses.
        crate::app::post_spawn::apply_post_spawn_setup(
            &mut **mux,
            cfg,
            pane_id,
            crate::app::post_spawn::PostSpawnArgs {
                theme,
                palette,
                cell_w,
                cell_h,
            },
        );

        // Local tab creation (mirrors create_initial_tab).
        let tab_id = self.session.alloc_tab_id();
        let tab = crate::session::Tab::new(tab_id, pane_id);
        self.session.add_tab(tab);
        if let Some(win) = self.session.get_window_mut(session_wid) {
            win.add_tab(tab_id);
        }

        log::info!(
            "default-terminal handoff: pane={pane_id} pid={client_pid:?} \
             title={title_for_log:?} icon={icon_for_log:?}"
        );

        Ok(())
    }

    /// Create an initial tab with one pane in the given mux window.
    ///
    /// The mux backend and window must already exist. The pane is stored
    /// inside the backend.
    pub(in crate::app) fn create_initial_tab(
        &mut self,
        session_wid: crate::session::WindowId,
        geom: InitialTabGeometry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let InitialTabGeometry {
            rows,
            cols,
            cell_w,
            cell_h,
        } = geom;
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);

        let config = SpawnConfig {
            cols,
            rows,
            scrollback: self.config.terminal.scrollback,
            shell_integration: self.config.behavior.shell_integration,
            shell: self.config.terminal.shell.clone(),
            ..SpawnConfig::default()
        };

        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);

        let cfg = &self.config;
        let mux = self.mux.as_mut().ok_or("mux backend missing")?;
        let pane_id = mux.spawn_pane(&config, theme)?;

        // Apply per-pane setup (theme, image config, bold-is-bright,
        // cell metrics) via the consolidated helper. Cell metrics seed
        // FixedPixels image placement coverage from the first frame.
        crate::app::post_spawn::apply_post_spawn_setup(
            &mut **mux,
            cfg,
            pane_id,
            crate::app::post_spawn::PostSpawnArgs {
                theme,
                palette,
                cell_w,
                cell_h,
            },
        );

        // Local tab creation.
        let tab_id = self.session.alloc_tab_id();
        let tab = crate::session::Tab::new(tab_id, pane_id);
        self.session.add_tab(tab);
        if let Some(win) = self.session.get_window_mut(session_wid) {
            win.add_tab(tab_id);
        }

        Ok(())
    }
}
