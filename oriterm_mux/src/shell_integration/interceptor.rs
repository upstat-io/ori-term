//! Raw VTE interceptor for sequences the high-level processor drops.
//!
//! The `vte::ansi::Processor` does not route OSC 133, OSC 9/99/777,
//! or XTVERSION (CSI >q) to `Handler` trait methods. This interceptor uses
//! a raw `vte::Parser` with a custom `Perform` impl to catch these sequences
//! before the high-level processor discards them. OSC 7 is also handled here
//! (with proper URI parsing and percent-decoding) because `Term` does NOT
//! override `Handler::set_working_directory` — the high-level handler default
//! is a no-op. The interceptor is therefore the sole canonical path for CWD
//! updates from OSC 7 (SSOT: `Term::set_cwd`).

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{Effect, HostEffect, NotificationSource, PtyEffect, PtyWriteKind};
use oriterm_core::{PromptState, Term};

/// Raw VTE interceptor state.
///
/// Borrows `Term<S>` fields mutably to update them in place. Runs on the
/// same locked terminal, on the same bytes, before the high-level processor.
pub(crate) struct RawInterceptor<'a, S: EffectSink> {
    term: &'a mut Term<S>,
}

impl<'a, S: EffectSink> RawInterceptor<'a, S> {
    /// Create a new interceptor targeting the given terminal.
    pub(crate) fn new(term: &'a mut Term<S>) -> Self {
        Self { term }
    }
}

impl<S: EffectSink> vte::Perform for RawInterceptor<'_, S> {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() || params[0].is_empty() {
            return;
        }
        match params[0] {
            // OSC 7 — Current working directory.
            // Format: OSC 7 ; file://hostname/path ST
            b"7" => self.handle_osc7(params),
            // OSC 133 — Semantic prompt markers.
            b"133" => self.handle_osc133(params),
            // OSC 633 — VS Code shell integration (superset of OSC 133).
            b"633" => self.handle_osc633(params),
            // OSC 9 / OSC 99 — iTerm2 / Kitty notifications.
            b"9" | b"99" => self.handle_notification_simple(params),
            // OSC 777 — rxvt-unicode notification.
            b"777" => self.handle_notification_777(params),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        // XTVERSION: CSI > q — report terminal name and version.
        if action == 'q' && intermediates == [b'>'] {
            let version = env!("CARGO_PKG_VERSION");
            let response = format!("\x1bP>|oriterm({version})\x1b\\");
            self.term.effect_sink().push(Effect::Pty(PtyEffect::Write {
                bytes: response.into_bytes(),
                kind: PtyWriteKind::Other,
            }));
        }
    }
}

impl<S: EffectSink> RawInterceptor<'_, S> {
    /// OSC 7: parse `file://hostname/path` URI and store the path.
    fn handle_osc7(&mut self, params: &[&[u8]]) {
        if params.len() < 2 {
            return;
        }
        let uri = std::str::from_utf8(params[1]).unwrap_or_default();
        let raw_path = parse_osc7_path(uri);
        if !raw_path.is_empty() {
            let path = percent_decode(raw_path).into_owned();
            self.term.set_cwd(Some(path.clone()));
            self.term.set_has_explicit_title(false);
            self.term.mark_title_dirty();
            self.term
                .effect_sink()
                .push(Effect::Host(HostEffect::CwdSet { cwd: path }));
        }
    }

    /// OSC 133: update prompt state machine and track command timing.
    ///
    /// `;A` / `;D` also restore the Kitty keyboard mode stack from any
    /// prior `;C` snapshot; `;C` takes the snapshot. See BUG-08-12 —
    /// prevents kitty modes pushed by a kitty-aware child from
    /// persisting past the next shell prompt when the child crashes or
    /// exits without popping cleanly.
    fn handle_osc133(&mut self, params: &[&[u8]]) {
        if params.len() < 2 || params[1].is_empty() {
            return;
        }
        self.handle_prompt_action(params[1][0]);
    }

    /// Shared A/B/C/D dispatch for both OSC 133 (xterm/bash/zsh shell
    /// integration) and OSC 633 (VS Code shell integration superset).
    /// Byte-identical control-flow collapsed to one SSOT per
    /// `impl-hygiene.md` §Algorithmic DRY — any future prompt-lifecycle
    /// change lands in one place.
    fn handle_prompt_action(&mut self, sub: u8) {
        match sub {
            b'A' => {
                // `;A` is the safety-net restore path for children that
                // crashed before `;D`.
                self.term.restore_keyboard_mode_stack();
                self.term.set_prompt_state(PromptState::PromptStart);
                self.term.set_prompt_mark_pending(true);
            }
            b'B' => {
                self.term.set_prompt_state(PromptState::CommandStart);
                self.term.set_command_start_mark_pending(true);
            }
            b'C' => {
                self.term.set_prompt_state(PromptState::OutputStart);
                self.term.set_command_start(std::time::Instant::now());
                self.term.set_output_start_mark_pending(true);
                // Snapshot AFTER `set_command_start` so the snapshot
                // captures the stack depth at command-start moment.
                self.term.snapshot_keyboard_mode_stack();
            }
            b'D' => {
                // `;D` is the clean-path restore — most programs reach it.
                self.term.restore_keyboard_mode_stack();
                self.term.set_prompt_state(PromptState::None);
                if let Some(duration) = self.term.finish_command(None) {
                    self.term
                        .effect_sink()
                        .push(Effect::Host(HostEffect::CommandComplete { duration }));
                }
            }
            _ => {}
        }
    }

    /// OSC 633: VS Code shell integration.
    ///
    /// Sub-commands mirror OSC 133 A/B/C/D semantics plus VS Code-specific
    /// extensions:
    /// - `A` — prompt start (like OSC 133;A).
    /// - `B` — command start (like OSC 133;B).
    /// - `C` — command executed (like OSC 133;C).
    /// - `D[;exitcode]` — command finished (like OSC 133;D).
    /// - `E;<line>` — raw command line text (stored via `Term::set_last_command_line`).
    /// - `P;<key>=<value>` — property setting. Only `Cwd=<path>` is honoured
    ///   and routes through `Term::set_cwd` (SSOT with OSC 7). Other keys
    ///   (`IsWindows=True`, `Task=...`, etc.) are recognised as opaque and
    ///   silently discarded.
    ///
    /// Source: `https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/common/xterm/shellIntegrationAddon.ts`.
    fn handle_osc633(&mut self, params: &[&[u8]]) {
        if params.len() < 2 || params[1].is_empty() {
            return;
        }
        match params[1][0] {
            sub @ (b'A' | b'B' | b'C' | b'D') => self.handle_prompt_action(sub),
            b'E' => {
                // OSC 633 ; E ; <command-line> — raw typed command text.
                let line = params
                    .get(2)
                    .map(|p| String::from_utf8_lossy(p).into_owned());
                self.term.set_last_command_line(line);
            }
            b'P' => {
                // OSC 633 ; P ; <key>=<value> — property setting.
                // Only `Cwd=<path>` is honoured; route through Term::set_cwd
                // to keep the CWD SSOT aligned with OSC 7. Also emits
                // `HostEffect::CwdSet` so downstream consumers (tab title,
                // status bar) receive CWD updates from VS Code shell
                // integration identically to OSC 7. Previously only OSC 7
                // pushed the effect — impl-hygiene F9 GAP.
                if let Some(pair) = params.get(2)
                    && let Some((key, value)) = split_key_value(pair)
                    && key == b"Cwd"
                {
                    let path = std::str::from_utf8(value).unwrap_or_default();
                    if !path.is_empty() {
                        self.term.set_cwd(Some(path.to_string()));
                        self.term.set_has_explicit_title(false);
                        self.term.mark_title_dirty();
                        self.term
                            .effect_sink()
                            .push(Effect::Host(HostEffect::CwdSet {
                                cwd: path.to_string(),
                            }));
                    }
                }
            }
            _ => {}
        }
    }

    /// OSC 9 / OSC 99: simple desktop notification.
    ///
    /// Wire-format differs between the two:
    /// - OSC 9 (Growl / iTerm2): `OSC 9 ; body ST` — payload at `params[1]`,
    ///   always carried in the `body` field (no title concept).
    /// - OSC 99 (Kitty): `OSC 99 ;; payload ST` (no metadata) or
    ///   `OSC 99 ; metadata ; payload ST` (with metadata). Payload at
    ///   `params[2]`. Per Kitty's OSC 99 spec the two semicolons are
    ///   mandatory even when metadata is empty. Metadata is a colon-separated
    ///   list of `key=value` pairs; only the `p` (payload type) key is
    ///   honoured. `p=title` (the default) routes payload to `title`;
    ///   `p=body` routes payload to `body`; other `p` values (`close`,
    ///   `icon`, `?`, `alive`, `buttons`) drop the notification per the
    ///   Kitty spec rule "Terminal emulators should ignore payloads of
    ///   unknown type to allow for future expansion of this protocol."
    ///
    /// Other metadata keys (`i` chunk id, `d` done flag, `e` base64-encoded,
    /// `t` notification type, `f` application name, `n` icon name, `o`
    /// urgency-on, `s` sound, `u` urgency, etc.) are recognised as opaque and
    /// silently discarded — chunking, base64, urgency, sound, and filtering
    /// are not honoured. Per the spec, a notification with neither title nor
    /// body is dropped.
    fn handle_notification_simple(&self, params: &[&[u8]]) {
        let source = if params.first().is_some_and(|p| *p == b"9") {
            NotificationSource::Osc9
        } else {
            NotificationSource::Osc99
        };
        let (title, body) = match source {
            NotificationSource::Osc9 => {
                let body = params
                    .get(1)
                    .map(|p| String::from_utf8_lossy(p).into_owned())
                    .unwrap_or_default();
                (String::new(), body)
            }
            NotificationSource::Osc99 => {
                let payload = params
                    .get(2)
                    .map(|p| String::from_utf8_lossy(p).into_owned())
                    .unwrap_or_default();
                let kind = parse_osc99_payload_kind(params.get(1).copied().unwrap_or(b""));
                let (title, body) = match kind {
                    Osc99PayloadKind::Title => (payload, String::new()),
                    Osc99PayloadKind::Body => (String::new(), payload),
                    Osc99PayloadKind::Unsupported => return,
                };
                // Per Kitty: "A notification with not title and no body is
                // ignored." OSC 9 has no such rule (empty body still fires).
                if title.is_empty() && body.is_empty() {
                    return;
                }
                (title, body)
            }
            NotificationSource::Osc777 => unreachable!("Osc777 routed to handle_notification_777"),
        };
        self.term
            .effect_sink()
            .push(Effect::Host(HostEffect::DesktopNotification {
                source,
                title,
                body,
            }));
    }

    /// OSC 777: rxvt-unicode notification (`notify;title;body`).
    fn handle_notification_777(&self, params: &[&[u8]]) {
        if params.len() < 2 {
            return;
        }
        let action = std::str::from_utf8(params[1]).unwrap_or_default();
        if action != "notify" {
            return;
        }
        let title = params
            .get(2)
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_default();
        let body = params
            .get(3)
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_default();
        self.term
            .effect_sink()
            .push(Effect::Host(HostEffect::DesktopNotification {
                source: NotificationSource::Osc777,
                title,
                body,
            }));
    }
}

/// Extract the filesystem path from an OSC 7 URI.
///
/// Handles formats like:
/// - `file://hostname/path/to/dir` → `/path/to/dir`
/// - `file:///path/to/dir` → `/path/to/dir` (empty hostname)
/// - `/path/to/dir` → `/path/to/dir` (no prefix)
///
/// Strips query strings (`?...`) and fragments (`#...`) from the path.
pub(super) fn parse_osc7_path(uri: &str) -> &str {
    let Some(after_scheme) = uri.strip_prefix("file://") else {
        return strip_uri_suffix(uri);
    };
    // After `file://`, we have `hostname/path`. Find the first `/`
    // which starts the absolute path (skipping the hostname portion).
    after_scheme
        .as_bytes()
        .iter()
        .position(|&b| b == b'/')
        .map_or(after_scheme, |pos| {
            let path = after_scheme.get(pos..).unwrap_or(after_scheme);
            strip_uri_suffix(path)
        })
}

/// Strip query string (`?...`) and fragment (`#...`) from a URI path.
fn strip_uri_suffix(path: &str) -> &str {
    let path = path.split('?').next().unwrap_or(path);
    path.split('#').next().unwrap_or(path)
}

/// Percent-decode a URI path component (`%20` → ` `, etc.).
///
/// Returns the input unchanged (borrowed) if no `%`-encoded sequences are
/// present. Otherwise allocates and returns an owned decoded string.
pub(super) fn percent_decode(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('%') {
        return std::borrow::Cow::Borrowed(s);
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(decoded) = decode_hex_pair(bytes[i + 1], bytes[i + 2]) {
                out.push(decoded);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out)
        .map(std::borrow::Cow::Owned)
        .unwrap_or(std::borrow::Cow::Borrowed(s))
}

/// Decode a pair of hex digits into a single byte.
fn decode_hex_pair(hi: u8, lo: u8) -> Option<u8> {
    let h = match hi {
        b'0'..=b'9' => hi - b'0',
        b'a'..=b'f' => hi - b'a' + 10,
        b'A'..=b'F' => hi - b'A' + 10,
        _ => return None,
    };
    let l = match lo {
        b'0'..=b'9' => lo - b'0',
        b'a'..=b'f' => lo - b'a' + 10,
        b'A'..=b'F' => lo - b'A' + 10,
        _ => return None,
    };
    Some(h << 4 | l)
}

/// Split an OSC 633 `P;<key>=<value>` pair on the first `=`.
///
/// Returns `None` when no `=` is present. Empty values return `Some((key, &[]))`.
fn split_key_value(pair: &[u8]) -> Option<(&[u8], &[u8])> {
    let eq = pair.iter().position(|&b| b == b'=')?;
    Some((&pair[..eq], &pair[eq + 1..]))
}

/// Kitty OSC 99 `p` (payload type) discriminator.
///
/// Per kitty `desktop-notifications.rst`:
/// > `p` — One of `title`, `body`, `close`, `icon`, `?`, `alive`, `buttons`.
/// > Default: `title`. Type of the payload. ... Terminal emulators should
/// > ignore payloads of unknown type to allow for future expansion of this
/// > protocol.
enum Osc99PayloadKind {
    /// Default — `p=title` or no `p` key.
    Title,
    /// `p=body`.
    Body,
    /// `p=close|icon|?|alive|buttons` or any unknown value — drop the
    /// notification per the spec's "ignore payloads of unknown type" rule.
    Unsupported,
}

/// Parse Kitty OSC 99 metadata for the `p=` key.
///
/// Metadata is a colon-separated list of `key=value` pairs. Only the `p` key
/// is honoured; all other keys (`i`, `d`, `e`, `t`, `f`, `n`, `o`, `s`, `u`,
/// `a`, `c`, `g`, `w`) are recognised as opaque and discarded — see the
/// `OSC-99` catalog row for the documented deviation. Empty metadata
/// (`OSC 99 ;; payload`) defaults to `Title` per spec.
fn parse_osc99_payload_kind(metadata: &[u8]) -> Osc99PayloadKind {
    for field in metadata.split(|&b| b == b':') {
        let mut parts = field.splitn(2, |&b| b == b'=');
        let key = parts.next().unwrap_or(&[]);
        let value = parts.next().unwrap_or(&[]);
        if key == b"p" {
            return match value {
                b"title" => Osc99PayloadKind::Title,
                b"body" => Osc99PayloadKind::Body,
                _ => Osc99PayloadKind::Unsupported,
            };
        }
    }
    Osc99PayloadKind::Title
}
