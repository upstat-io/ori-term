//! OSC dispatch handler.

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;
use core::str;

use core::str::FromStr;

use cursor_icon::CursorIcon;
use log::debug;

use crate::ansi::colors::{parse_number, xparse_color, Hyperlink};
use crate::ansi::handler::Handler;
use crate::ansi::types::{CursorShape, NamedColor};

/// Dispatch an OSC escape sequence to the handler.
pub(super) fn dispatch<H: Handler>(handler: &mut H, params: &[&[u8]], bell_terminated: bool) {
    let terminator = if bell_terminated { "\x07" } else { "\x1b\\" };

    fn unhandled(params: &[&[u8]]) {
        let mut buf = String::new();
        for items in params {
            buf.push('[');
            for item in *items {
                let _ = write!(buf, "{:?}", *item as char);
            }
            buf.push_str("],");
        }
        debug!("[unhandled osc_dispatch]: [{}] at line {}", &buf, line!());
    }

    if params.is_empty() || params[0].is_empty() {
        return;
    }

    match params[0] {
        // Set window title and/or icon name.
        b"0" | b"1" | b"2" => {
            if params.len() >= 2 {
                let text = params[1..]
                    .iter()
                    .flat_map(|x| str::from_utf8(x))
                    .collect::<Vec<&str>>()
                    .join(";")
                    .trim()
                    .to_owned();
                match params[0] {
                    b"0" => {
                        handler.set_title(Some(text.clone()));
                        handler.set_icon_name(Some(text));
                    }
                    b"1" => {
                        handler.set_icon_name(Some(text));
                    }
                    _ => {
                        handler.set_title(Some(text));
                    }
                }
                return;
            }
            unhandled(params);
        },

        // Set working directory (shell integration).
        b"7" => {
            if params.len() >= 2 {
                let uri = params[1..]
                    .iter()
                    .flat_map(|x| str::from_utf8(x))
                    .collect::<Vec<&str>>()
                    .join(";")
                    .trim()
                    .to_owned();
                if uri.is_empty() {
                    handler.set_working_directory(None);
                } else {
                    handler.set_working_directory(Some(uri));
                }
                return;
            }
            unhandled(params);
        },

        // Set color index.
        b"4" => {
            if params.len() <= 1 || params.len() % 2 == 0 {
                unhandled(params);
                return;
            }

            for chunk in params[1..].chunks(2) {
                let index = match parse_number(chunk[0]) {
                    Some(index) => index,
                    None => {
                        unhandled(params);
                        continue;
                    },
                };

                if let Some(c) = xparse_color(chunk[1]) {
                    handler.set_color(index as usize, c);
                } else if chunk[1] == b"?" {
                    let prefix = format!("4;{index}");
                    handler.dynamic_color_sequence(prefix, index as usize, terminator);
                } else {
                    unhandled(params);
                }
            }
        },

        // Hyperlink.
        b"8" if params.len() > 2 => {
            let link_params = params[1];

            // NOTE: The escape sequence is of form 'OSC 8 ; params ; URI ST', where
            // URI is URL-encoded. However `;` is a special character and might be
            // passed as is, thus we need to rebuild the URI.
            let mut uri = str::from_utf8(params[2]).unwrap_or_default().to_string();
            for param in params[3..].iter() {
                uri.push(';');
                uri.push_str(str::from_utf8(param).unwrap_or_default());
            }

            // The OSC 8 escape sequence must be stopped when getting an empty `uri`.
            if uri.is_empty() {
                handler.set_hyperlink(None);
                return;
            }

            // Link parameters are in format of `key1=value1:key2=value2`. Currently only
            // key `id` is defined.
            let id = link_params
                .split(|&b| b == b':')
                .find_map(|kv| kv.strip_prefix(b"id="))
                .and_then(|kv| str::from_utf8(kv).ok().map(|e| e.to_owned()));

            handler.set_hyperlink(Some(Hyperlink { id, uri }));
        },

        // Get/set Foreground, Background, Cursor colors.
        b"10" | b"11" | b"12" => {
            if params.len() >= 2 {
                if let Some(mut dynamic_code) = parse_number(params[0]) {
                    for param in &params[1..] {
                        // 10 is the first dynamic color, also the foreground.
                        let offset = dynamic_code as usize - 10;
                        let index = NamedColor::Foreground as usize + offset;

                        // End of setting dynamic colors.
                        if index > NamedColor::Cursor as usize {
                            unhandled(params);
                            break;
                        }

                        if let Some(color) = xparse_color(param) {
                            handler.set_color(index, color);
                        } else if param == b"?" {
                            handler.dynamic_color_sequence(
                                dynamic_code.to_string(),
                                index,
                                terminator,
                            );
                        } else {
                            unhandled(params);
                        }
                        dynamic_code += 1;
                    }
                    return;
                }
            }
            unhandled(params);
        },

        // Set mouse cursor shape.
        b"22" if params.len() == 2 => {
            let shape = String::from_utf8_lossy(params[1]);
            match CursorIcon::from_str(&shape) {
                Ok(cursor_icon) => handler.set_mouse_cursor_icon(cursor_icon),
                Err(_) => debug!("[osc 22] unrecognized cursor icon shape: {shape:?}"),
            }
        },

        // Set cursor style.
        b"50" => {
            if params.len() >= 2
                && params[1].len() >= 13
                && params[1][0..12] == *b"CursorShape="
            {
                let shape = match params[1][12] as char {
                    '0' => CursorShape::Block,
                    '1' => CursorShape::Beam,
                    '2' => CursorShape::Underline,
                    _ => return unhandled(params),
                };
                handler.set_cursor_shape(shape);
                return;
            }
            unhandled(params);
        },

        // Set clipboard.
        b"52" => {
            if params.len() < 3 {
                return unhandled(params);
            }

            let clipboard = params[1].first().unwrap_or(&b'c');
            match params[2] {
                b"?" => handler.clipboard_load(*clipboard, terminator),
                base64 => handler.clipboard_store(*clipboard, base64),
            }
        },

        // Reset color index.
        b"104" => {
            // Reset all color indexes when no parameters are given.
            if params.len() == 1 || params[1].is_empty() {
                for i in 0..256 {
                    handler.reset_color(i);
                }
                return;
            }

            // Reset color indexes given as parameters.
            for param in &params[1..] {
                match parse_number(param) {
                    Some(index) => handler.reset_color(index as usize),
                    None => unhandled(params),
                }
            }
        },

        // Reset foreground color.
        b"110" => handler.reset_color(NamedColor::Foreground as usize),

        // Reset background color.
        b"111" => handler.reset_color(NamedColor::Background as usize),

        // Reset text cursor color.
        b"112" => handler.reset_color(NamedColor::Cursor as usize),

        // iTerm2 proprietary sequences.
        b"1337" => {
            if params.len() < 2 {
                unhandled(params);
                return;
            }
            dispatch_iterm2_osc1337(handler, &params[1..]);
        },

        _ => unhandled(params),
    }
}

/// Route an OSC 1337 sub-command to the matching `Handler` method.
///
/// The first sub-op is expected to be `key[=value]`. `File=...` still routes
/// through `Handler::iterm2_file` — the full param tail is forwarded so the
/// existing iTerm2 image parser remains unchanged. Non-image sub-ops
/// (`SetMark`, `RemoteHost=`, `CurrentDir=`, `Copy=`, `ReportCellSize`,
/// `SetUserVar=`, `ShellIntegrationVersion=`) route to their own dedicated
/// `Handler` hooks so each consumer (e.g. `Term`) can implement them without
/// repeating the `key[=value]` parse.
fn dispatch_iterm2_osc1337<H: Handler>(handler: &mut H, params: &[&[u8]]) {
    // `params[0]` is the first sub-op, e.g. `SetMark`, `RemoteHost=user@host`,
    // `File=name=...`, etc.
    let head = params[0];

    if head.starts_with(b"File=") {
        handler.iterm2_file(params);
        return;
    }

    // Split `head` on the first `=` into `(key, value)`.
    let (key, value) = match head.iter().position(|&b| b == b'=') {
        Some(idx) => (&head[..idx], Some(&head[idx + 1..])),
        None => (head, None),
    };

    match key {
        b"SetMark" => handler.iterm2_set_mark(),
        b"ReportCellSize" => handler.iterm2_report_cell_size(),
        b"RemoteHost" => {
            if let Some(v) = value {
                handler.iterm2_remote_host(v);
            }
        },
        b"CurrentDir" => {
            if let Some(v) = value {
                handler.iterm2_current_dir(v);
            }
        },
        b"Copy" => {
            if let Some(v) = value {
                handler.iterm2_copy(v);
            }
        },
        b"ShellIntegrationVersion" => {
            if let Some(v) = value {
                handler.iterm2_shell_integration_version(v);
            }
        },
        b"SetUserVar" => {
            // `SetUserVar=NAME=<base64-value>` — the head split above gave us
            // `value = Some(b"NAME=<base64>")`. Split once more on `=` to
            // recover the name and the base64-encoded value.
            let Some(rest) = value else { return };
            let Some(sep) = rest.iter().position(|&b| b == b'=') else {
                return;
            };
            let name = &rest[..sep];
            let encoded = &rest[sep + 1..];
            handler.iterm2_set_user_var(name, encoded);
        },
        _ => {},
    }
}
