//! Shared content typography and layout helpers for all settings pages.
//!
//! Defines the page header, section header, and body spacing primitives
//! that every settings page builder imports. Extracted from `appearance.rs`
//! so the shared content-layout path has proper ownership.

use oriterm_ui::geometry::Insets;
use oriterm_ui::layout::{Align, SizeSpec};
use oriterm_ui::text::{FontWeight, TextTransform};
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::label::{LabelStyle, LabelWidget};
use oriterm_ui::widgets::scroll::ScrollWidget;
use oriterm_ui::widgets::scrollbar::ScrollbarStyle;
use oriterm_ui::widgets::separator::{SeparatorStyle, SeparatorWidget};
use oriterm_ui::widgets::spacer::SpacerWidget;

/// Page content padding (shared by all page builders).
pub(super) const PAGE_PADDING: Insets = Insets::vh(0.0, 28.0);

/// Gap between sections (shared by all page builders).
pub(super) const SECTION_GAP: f32 = 28.0;

/// Page title font size.
pub(super) const TITLE_FONT_SIZE: f32 = 18.0;

/// Page description font size.
pub(super) const DESC_FONT_SIZE: f32 = 12.0;

/// Section header font size.
pub(super) const SECTION_FONT_SIZE: f32 = 11.0;

/// Letter spacing for page titles (matches mockup `letter-spacing: 0.05em`).
pub(super) const TITLE_LETTER_SPACING: f32 = 0.9;

/// Letter spacing for section headers (matches mockup `letter-spacing: 0.15em` at 11px).
pub(super) const SECTION_LETTER_SPACING: f32 = 1.65;

/// Builds a settings page with a sticky header and scrollable body.
///
/// The header (title + description) stays fixed at the top while sections
/// scroll beneath it. All 8 settings pages use this shared layout.
pub(super) fn build_settings_page(
    title_text: &str,
    desc_text: &str,
    sections: Vec<Box<dyn Widget>>,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let header = build_page_header(title_text, desc_text, theme);

    let mut body = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_padding(Insets::tlbr(
            0.0,
            PAGE_PADDING.left,
            28.0,
            PAGE_PADDING.right,
        ))
        .with_gap(SECTION_GAP);
    for section in sections {
        body = body.with_child(section);
    }

    let mut scroll = ScrollWidget::vertical(Box::new(body))
        .with_scrollbar_style(ScrollbarStyle::from_theme(theme));
    scroll.set_height(SizeSpec::Fill);

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fill)
            .with_child(header)
            .with_child(Box::new(scroll)),
    )
}

/// Page header: title + description with fixed positioning.
pub(super) fn build_page_header(
    title_text: &str,
    desc_text: &str,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let title = LabelWidget::new(title_text).with_style(LabelStyle {
        font_size: TITLE_FONT_SIZE,
        weight: FontWeight::BOLD,
        letter_spacing: TITLE_LETTER_SPACING,
        color: theme.fg_bright,
        text_transform: TextTransform::Uppercase,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });
    let desc = LabelWidget::new(desc_text).with_style(LabelStyle {
        font_size: DESC_FONT_SIZE,
        color: theme.fg_secondary,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });
    Box::new(
        ContainerWidget::column()
            .with_gap(4.0)
            .with_width(SizeSpec::Fill)
            .with_padding(Insets::tlbr(24.0, 28.0, 20.0, 28.0))
            .with_child(Box::new(title))
            .with_child(Box::new(desc)),
    )
}

/// Builds a section header column: title row + 12px spacer.
///
/// The title row renders `// TITLE ─────────` with the prefix and title as
/// separate labels so each gets distinct letter spacing. The 12px spacer
/// provides consistent title-to-first-row spacing (matching the mockup's
/// `margin-bottom: 12px` on `.section-title`).
pub(super) fn build_section_header(title: &str, theme: &UiTheme) -> Box<dyn Widget> {
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_title_row(title, theme))
            .with_child(Box::new(SpacerWidget::fixed(0.0, 12.0))),
    )
}

/// Builds a section header column with an optional description block.
///
/// Layout: title row → 4px gap → description label → 12px spacer.
/// The description uses `12px` body text, muted color, and `line_height: 1.5`.
pub(super) fn build_section_header_with_description(
    title: &str,
    desc: &str,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let desc_label = LabelWidget::new(desc).with_style(LabelStyle {
        font_size: DESC_FONT_SIZE,
        color: theme.fg_secondary,
        line_height: Some(1.5),
        ..LabelStyle::from_theme(theme)
    });
    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_title_row(title, theme))
            .with_child(Box::new(SpacerWidget::fixed(0.0, 4.0)))
            .with_child(Box::new(desc_label))
            .with_child(Box::new(SpacerWidget::fixed(0.0, 12.0))),
    )
}

/// Builds the `// TITLE ─────────` row with split prefix/title labels.
///
/// The `"//"` prefix uses zero letter spacing while the title text uses
/// `SECTION_LETTER_SPACING` (1.65px). Both use `FontWeight::MEDIUM` (500).
fn build_section_title_row(title: &str, theme: &UiTheme) -> Box<dyn Widget> {
    let prefix = LabelWidget::new("//").with_style(LabelStyle {
        font_size: SECTION_FONT_SIZE,
        weight: FontWeight::MEDIUM,
        letter_spacing: 0.0,
        color: theme.fg_faint,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });
    let title_label = LabelWidget::new(title).with_style(LabelStyle {
        font_size: SECTION_FONT_SIZE,
        weight: FontWeight::MEDIUM,
        letter_spacing: SECTION_LETTER_SPACING,
        color: theme.fg_faint,
        text_transform: TextTransform::Uppercase,
        line_height: None,
        ..LabelStyle::from_theme(theme)
    });
    let rule = SeparatorWidget::horizontal().with_style(SeparatorStyle {
        thickness: 2.0,
        color: theme.border,
        ..SeparatorStyle::from_theme(theme)
    });
    Box::new(
        ContainerWidget::row()
            .with_width(SizeSpec::Fill)
            .with_align(Align::Center)
            .with_gap(10.0)
            .with_child(Box::new(prefix))
            .with_child(Box::new(title_label))
            .with_child(Box::new(rule)),
    )
}

#[cfg(test)]
mod tests;
