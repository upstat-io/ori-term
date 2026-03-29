//! Keymap data structure mapping keystrokes to semantic actions.
//!
//! A `Keymap` holds a list of `KeyBinding`s. Each binding maps a `Keystroke`
//! (key + modifiers) to a `KeymapAction`, optionally scoped to a context
//! string (e.g., `"Button"`, `"Dropdown"`). The `lookup()` method finds the
//! best matching action given a keystroke and the current context stack.

use crate::action::keymap_action::{
    Activate, Confirm, DecrementValue, Dismiss, FocusNext, FocusPrev, IncrementValue, KeymapAction,
    NavigateDown, NavigateUp, ValueToMax, ValueToMin,
};
use crate::input::{Key, Modifiers};

/// A keystroke pattern: a key combined with modifier flags.
///
/// Used as the matching key in keymap lookups. Reuses `Key` and `Modifiers`
/// from `oriterm_ui::input`. Derives `Hash` for potential future `HashMap`
/// acceleration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Keystroke {
    /// The key pressed.
    pub key: Key,
    /// Active modifier flags.
    pub modifiers: Modifiers,
}

impl Keystroke {
    /// Creates a new keystroke.
    pub const fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

/// A single keymap binding: keystroke -> action, optionally scoped.
pub struct KeyBinding {
    /// The keystroke that triggers this binding.
    pub keystroke: Keystroke,
    /// The action to dispatch when matched.
    action: Box<dyn KeymapAction>,
    /// Optional context scope (e.g., `"Button"`, `"Dropdown"`).
    ///
    /// `None` means the binding is global (matches in any context).
    /// `Some("X")` means it only matches when `"X"` is in the context stack.
    pub context: Option<&'static str>,
}

impl KeyBinding {
    /// Creates a new key binding.
    pub fn new(
        keystroke: Keystroke,
        action: impl KeymapAction,
        context: Option<&'static str>,
    ) -> Self {
        Self {
            keystroke,
            action: Box::new(action),
            context,
        }
    }
}

impl std::fmt::Debug for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyBinding")
            .field("keystroke", &self.keystroke)
            .field("action", &self.action.name())
            .field("context", &self.context)
            .finish()
    }
}

/// Maps keystrokes to semantic actions with context-scoped precedence.
///
/// Bindings are evaluated in order. When multiple bindings match the same
/// keystroke, context-scoped bindings win over global (`context: None`).
/// Among context-scoped bindings, the one matching the deepest context
/// in the stack wins.
#[derive(Default)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

impl Keymap {
    /// Creates an empty keymap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a keymap with the default widget keybindings.
    ///
    /// These cover all existing keyboard-activated widgets:
    /// - Enter/Space -> Activate (scoped to `"Button"`)
    /// - Tab/Shift+Tab -> FocusNext/FocusPrev (global)
    /// - Arrow keys -> navigation (scoped per widget type)
    /// - Enter/Space -> Confirm (scoped to `"Dropdown"` and `"Menu"`)
    /// - Escape -> Dismiss (scoped to `"Dropdown"`, `"Menu"`, `"Dialog"`)
    /// - Slider keys (scoped to `"Slider"`)
    // `vec![]` can't be used because `KeyBinding::new` takes `impl KeymapAction`
    // and each action is a different concrete type — monomorphization prevents
    // unifying them in a single `vec![]` expression.
    pub fn defaults() -> Self {
        let mut b = Vec::with_capacity(22);
        Self::push_button_bindings(&mut b);
        Self::push_focus_bindings(&mut b);
        Self::push_list_bindings(&mut b, "Dropdown");
        Self::push_list_bindings(&mut b, "Menu");
        Self::push_dialog_bindings(&mut b);
        Self::push_slider_bindings(&mut b);
        Self { bindings: b }
    }

    /// Finds the best-matching action for a keystroke given a context stack.
    ///
    /// Returns `None` if no binding matches. When multiple bindings match
    /// the same keystroke, context-scoped bindings win over `context: None`.
    /// Among context-scoped bindings, deeper context (later in stack) wins.
    pub fn lookup(
        &self,
        keystroke: Keystroke,
        context_stack: &[&str],
    ) -> Option<Box<dyn KeymapAction>> {
        let mut best: Option<(i32, &KeyBinding)> = None;

        for binding in &self.bindings {
            if binding.keystroke != keystroke {
                continue;
            }

            let score = match binding.context {
                None => 0, // Global binding = lowest priority.
                Some(ctx) => {
                    // Find the deepest position in the stack (1-indexed for scoring).
                    match context_stack.iter().rposition(|c| *c == ctx) {
                        Some(pos) => {
                            // +1 so any context match beats global (score 0).
                            i32::try_from(pos).unwrap_or(i32::MAX - 1) + 1
                        }
                        None => continue, // Required context not in stack — skip.
                    }
                }
            };

            match &best {
                Some((best_score, _)) if score <= *best_score => {}
                _ => best = Some((score, binding)),
            }
        }

        best.map(|(_, binding)| binding.action.boxed_clone())
    }

    /// Adds or replaces a binding (user override).
    ///
    /// If a binding with the same keystroke and context already exists,
    /// it is replaced. Otherwise the new binding is appended.
    pub fn rebind(&mut self, binding: KeyBinding) {
        if let Some(existing) = self
            .bindings
            .iter_mut()
            .find(|b| b.keystroke == binding.keystroke && b.context == binding.context)
        {
            *existing = binding;
        } else {
            self.bindings.push(binding);
        }
    }

    // Button activation (scoped — NOT global, to avoid intercepting text input).
    fn push_button_bindings(b: &mut Vec<KeyBinding>) {
        let m = Modifiers::NONE;
        b.push(KeyBinding::new(
            Keystroke::new(Key::Enter, m),
            Activate,
            Some("Button"),
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Space, m),
            Activate,
            Some("Button"),
        ));
    }

    // Focus traversal (global — Tab is not consumed by text controllers).
    fn push_focus_bindings(b: &mut Vec<KeyBinding>) {
        b.push(KeyBinding::new(
            Keystroke::new(Key::Tab, Modifiers::NONE),
            FocusNext,
            None,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Tab, Modifiers::SHIFT_ONLY),
            FocusPrev,
            None,
        ));
    }

    // List-style navigation (shared by Dropdown and Menu contexts).
    fn push_list_bindings(b: &mut Vec<KeyBinding>, ctx: &'static str) {
        let m = Modifiers::NONE;
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowDown, m),
            NavigateDown,
            Some(ctx),
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowUp, m),
            NavigateUp,
            Some(ctx),
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Enter, m),
            Confirm,
            Some(ctx),
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Space, m),
            Confirm,
            Some(ctx),
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Escape, m),
            Dismiss,
            Some(ctx),
        ));
    }

    fn push_dialog_bindings(b: &mut Vec<KeyBinding>) {
        b.push(KeyBinding::new(
            Keystroke::new(Key::Escape, Modifiers::NONE),
            Dismiss,
            Some("Dialog"),
        ));
    }

    fn push_slider_bindings(b: &mut Vec<KeyBinding>) {
        let m = Modifiers::NONE;
        let ctx = Some("Slider");
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowRight, m),
            IncrementValue,
            ctx,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowUp, m),
            IncrementValue,
            ctx,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowLeft, m),
            DecrementValue,
            ctx,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::ArrowDown, m),
            DecrementValue,
            ctx,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::Home, m),
            ValueToMin,
            ctx,
        ));
        b.push(KeyBinding::new(
            Keystroke::new(Key::End, m),
            ValueToMax,
            ctx,
        ));
    }
}

impl std::fmt::Debug for Keymap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keymap")
            .field("bindings", &self.bindings.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
