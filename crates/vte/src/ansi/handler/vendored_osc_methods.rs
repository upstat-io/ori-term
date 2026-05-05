//! Vendored-patch OSC `Handler` trait method declarations.
//!
//! Defines `handler_vendored_osc_methods!()` — a `macro_rules!` that expands
//! to the oriterm vendored-patch OSC trait methods (Section 10.0 OSC 1337
//! iTerm2 sub-ops + Section 10.9 OSC 3/5/6/13/14/17/19/113/114/117/119
//! color + X11 property extensions).
//!
//! Kept separate from `core_methods.rs` so upstream-pull churn never
//! touches the vendored set and file-size stays under the hygiene cap.

macro_rules! handler_vendored_osc_methods {
    () => {
 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; SetMark — iTerm2 navigation mark.
 ///
 /// Equivalent to OSC 133;A's prompt boundary: records the current
 /// cursor row so the user can jump between marks in the scrollback.
        fn iterm2_set_mark(&mut self) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; RemoteHost=user@host — iTerm2 remote host identity.
        fn iterm2_remote_host(&mut self, _host: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; CurrentDir=/path — iTerm2 CWD update.
 ///
 /// Shares the canonical CWD field with OSC 7 (SSOT: `Term::set_cwd`).
        fn iterm2_current_dir(&mut self, _path: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; Copy=:<base64> — iTerm2 clipboard copy.
        fn iterm2_copy(&mut self, _data: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; ReportCellSize — iTerm2 request for cell pixel size.
 ///
 /// Handler replies over PTY with the current cell dimensions.
        fn iterm2_report_cell_size(&mut self) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; SetUserVar=NAME=<base64> — iTerm2 user-variable.
        fn iterm2_set_user_var(&mut self, _name: &[u8], _value: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 1337 non-image sub-op (Section 10.0).
 /// Handle OSC 1337 ; ShellIntegrationVersion=N — iTerm2 shell-integration version.
        fn iterm2_shell_integration_version(&mut self, _version: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 3 — set X11 window property (Section 10.9).
 /// OSC 3 ; Pt — Pt is a single payload in the form `prop[=value]`.
 ///
 /// With `=value` present the handler stores `prop → value`; bare
 /// `prop` deletes the property. Platform-specific (X11 window-manager
 /// hint); on non-X11 platforms the handler records state without
 /// side effects.
        fn set_x11_property(&mut self, _payload: &[u8]) {}

 // VENDORED PATCH (oriterm): OSC 5 — special color set (Section 10.9).
 /// OSC 5 ; Ps ; spec — set one of the special-color slots (bold,
 /// underline, blink, reverse, italics — per xterm ctlseqs).
        fn set_special_color(&mut self, _index: usize, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 5 — special color query (Section 10.9).
 /// OSC 5 ; Ps ; ? — query one of the special-color slots. The handler
 /// replies over PTY with the current color.
        fn query_special_color(&mut self, _index: usize, _terminator: &str) {}

 // VENDORED PATCH (oriterm): OSC 6 — tab title color (Section 10.9).
 /// OSC 6 ; spec — iTerm2 "Change Title Tab Color". Accepts the
 /// standard xterm color spec (`rgb:RR/GG/BB` or `#RRGGBB`).
        fn set_tab_title_color(&mut self, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 13 — mouse foreground color (Section 10.9).
 /// OSC 13 ; spec — set mouse-cursor foreground color.
        fn set_mouse_fg_color(&mut self, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 14 — mouse background color (Section 10.9).
 /// OSC 14 ; spec — set mouse-cursor background color.
        fn set_mouse_bg_color(&mut self, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 17 — highlight background color (Section 10.9).
 /// OSC 17 ; spec — set selection highlight background color.
        fn set_highlight_bg_color(&mut self, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 19 — highlight foreground color (Section 10.9).
 /// OSC 19 ; spec — set selection highlight foreground color.
        fn set_highlight_fg_color(&mut self, _color: Rgb) {}

 // VENDORED PATCH (oriterm): OSC 13 — mouse foreground query (Section 10.9).
 /// OSC 13 ; ? — query mouse-cursor foreground color. Handler replies
 /// over PTY with the current color.
        fn query_mouse_fg_color(&mut self, _terminator: &str) {}

 // VENDORED PATCH (oriterm): OSC 14 — mouse background query (Section 10.9).
 /// OSC 14 ; ? — query mouse-cursor background color.
        fn query_mouse_bg_color(&mut self, _terminator: &str) {}

 // VENDORED PATCH (oriterm): OSC 17 — highlight background query (Section 10.9).
 /// OSC 17 ; ? — query selection highlight background color.
        fn query_highlight_bg_color(&mut self, _terminator: &str) {}

 // VENDORED PATCH (oriterm): OSC 19 — highlight foreground query (Section 10.9).
 /// OSC 19 ; ? — query selection highlight foreground color.
        fn query_highlight_fg_color(&mut self, _terminator: &str) {}

 // VENDORED PATCH (oriterm): OSC 113 — reset mouse fg (Section 10.9).
 /// OSC 113 — reset mouse-cursor foreground color to the palette default.
        fn reset_mouse_fg_color(&mut self) {}

 // VENDORED PATCH (oriterm): OSC 114 — reset mouse bg (Section 10.9).
 /// OSC 114 — reset mouse-cursor background color to the palette default.
        fn reset_mouse_bg_color(&mut self) {}

 // VENDORED PATCH (oriterm): OSC 117 — reset highlight bg (Section 10.9).
 /// OSC 117 — reset selection highlight background color.
        fn reset_highlight_bg_color(&mut self) {}

 // VENDORED PATCH (oriterm): OSC 119 — reset highlight fg (Section 10.9).
 /// OSC 119 — reset selection highlight foreground color.
        fn reset_highlight_fg_color(&mut self) {}
    };
}

pub(crate) use handler_vendored_osc_methods;
