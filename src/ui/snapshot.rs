use std::collections::BTreeMap;
use std::path::Path;

use chrono::{Duration, NaiveDate};
use clap::ValueEnum;
use color_eyre::eyre::{Context, Result};
use image::ImageFormat;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::analytics::AnalyticsSnapshot;
use crate::ui::export::render_share_card;
use crate::ui::theme::Theme;
use crate::utils::formatting::{format_exact_tokens, format_price_summary};
use crate::utils::time::TimeRange;

const DAILY_BAR_WIDTH: usize = 24;
const TABLE_LABEL_WIDTH: usize = 24;

/// The destination format emitted by `shaw-oc-stats snapshot`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum SnapshotFormat {
    /// Print an ASCII-art snapshot directly to standard output.
    #[default]
    #[value(name = "terminal", aliases = ["ascii", "text"])]
    Terminal,
    /// Write the same snapshot as a PNG share card.
    #[value(name = "image", aliases = ["png"])]
    Image,
}

/// The section or sections emitted by `shaw-oc-stats snapshot`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum SnapshotView {
    /// A compact usage overview.
    Overview,
    /// A per-day token chart with the exact value printed for every bar.
    Daily,
    /// Usage totals grouped by model.
    Model,
    /// The overview, daily chart, model table, and provider table.
    #[default]
    All,
}

pub fn render_snapshot(
    snapshot: &AnalyticsSnapshot,
    range: TimeRange,
    view: SnapshotView,
) -> String {
    let mut sections = Vec::new();

    if matches!(view, SnapshotView::Overview | SnapshotView::All) {
        sections.push(render_overview(snapshot, range));
    }
    if matches!(view, SnapshotView::Daily | SnapshotView::All) {
        sections.push(render_daily_chart(snapshot, range));
    }
    if matches!(view, SnapshotView::Model | SnapshotView::All) {
        sections.push(render_models(snapshot));
    }
    if view == SnapshotView::All {
        sections.push(render_providers(snapshot));
    }

    sections.join("\n\n")
}

pub fn write_snapshot_image(
    snapshot: &AnalyticsSnapshot,
    range: TimeRange,
    view: SnapshotView,
    theme: &Theme,
    output_path: &Path,
) -> Result<()> {
    let text = render_snapshot(snapshot, range, view);
    let buffer = snapshot_buffer(&text);
    let image = render_share_card(&buffer, theme)?;
    image
        .save_with_format(output_path, ImageFormat::Png)
        .wrap_err_with(|| {
            format!(
                "failed to write snapshot image to {}",
                output_path.display()
            )
        })
}

fn render_overview(snapshot: &AnalyticsSnapshot, range: TimeRange) -> String {
    let overview = &snapshot.overview;
    boxed_section(
        &format!("OPENCODE USAGE SNAPSHOT ({})", range.label()),
        &[
            format!(
                "Total tokens : {}",
                format_exact_tokens(overview.total_tokens)
            ),
            format!(
                "Input tokens : {}",
                format_exact_tokens(overview.input_tokens)
            ),
            format!(
                "Output tokens: {}",
                format_exact_tokens(overview.output_tokens)
            ),
            format!(
                "Cache tokens : {}",
                format_exact_tokens(overview.cache_tokens)
            ),
            format!(
                "Total cost   : {}",
                format_price_summary(&overview.total_cost)
            ),
            format!("Sessions     : {}", overview.sessions),
            format!("Messages     : {}", overview.messages),
            format!("Prompts      : {}", overview.prompts),
            format!("Models used  : {}", overview.models_used),
            format!("Active days  : {}", overview.active_days),
        ],
    )
}

fn render_daily_chart(snapshot: &AnalyticsSnapshot, range: TimeRange) -> String {
    let totals = snapshot
        .daily
        .iter()
        .map(|day| (day.date, day.tokens.total()))
        .collect::<BTreeMap<_, _>>();
    let dates = chart_dates(
        &snapshot.daily,
        range,
        crate::utils::time::current_local_date(),
    );
    if dates.is_empty() {
        return boxed_section(
            "DAILY TOKEN CHART (EXACT VALUES)",
            &["No activity in this time range.".to_string()],
        );
    }

    let maximum = dates
        .iter()
        .map(|date| totals.get(date).copied().unwrap_or_default())
        .max()
        .unwrap_or_default();
    let mut lines = vec!["Date          Tokens        Usage".to_string()];

    for date in dates {
        let total = totals.get(&date).copied().unwrap_or_default();
        let bar = daily_bar(total, maximum);
        lines.push(format!(
            "{}  {:>12}  {bar}",
            date,
            format_exact_tokens(total),
        ));
    }

    boxed_section("DAILY TOKEN CHART (EXACT VALUES)", &lines)
}

fn chart_dates(
    daily: &[crate::analytics::daily::DailyUsage],
    range: TimeRange,
    today: NaiveDate,
) -> Vec<NaiveDate> {
    let Some(start) = range.start_date(today) else {
        return daily.iter().map(|day| day.date).collect();
    };
    let mut dates = Vec::new();
    let mut date = start;
    while date <= today {
        dates.push(date);
        date += Duration::days(1);
    }
    dates
}

fn daily_bar(total: u64, maximum: u64) -> String {
    if total == 0 || maximum == 0 {
        return String::new();
    }

    let length = ((total as f64 / maximum as f64) * DAILY_BAR_WIDTH as f64)
        .ceil()
        .clamp(1.0, DAILY_BAR_WIDTH as f64) as usize;
    "#".repeat(length)
}

fn render_models(snapshot: &AnalyticsSnapshot) -> String {
    if snapshot.models.is_empty() {
        return boxed_section(
            "MODEL USAGE (EXACT TOKEN TOTALS)",
            &["No model activity in this time range.".to_string()],
        );
    }

    let mut lines = vec![format!(
        "{:<TABLE_LABEL_WIDTH$} {:>12} {:>7} {:>12} {:>12} {:>12}  Cost",
        "Model", "Tokens", "Share", "Input", "Output", "Cache"
    )];
    for model in &snapshot.models {
        lines.push(format!(
            "{:<TABLE_LABEL_WIDTH$} {:>12} {:>6.2}% {:>12} {:>12} {:>12}  {}",
            truncate_table_label(&model.model_id),
            format_exact_tokens(model.total_tokens),
            model.percentage,
            format_exact_tokens(model.input_tokens),
            format_exact_tokens(model.output_tokens),
            format_exact_tokens(model.cache_tokens),
            format_price_summary(&model.cost),
        ));
    }
    boxed_section("MODEL USAGE (EXACT TOKEN TOTALS)", &lines)
}

fn render_providers(snapshot: &AnalyticsSnapshot) -> String {
    if snapshot.providers.is_empty() {
        return boxed_section(
            "PROVIDER USAGE (EXACT TOKEN TOTALS)",
            &["No provider activity in this time range.".to_string()],
        );
    }

    let mut lines = vec![format!(
        "{:<TABLE_LABEL_WIDTH$} {:>12} {:>7} {:>12} {:>12} {:>12}  Cost",
        "Provider", "Tokens", "Share", "Input", "Output", "Cache"
    )];
    for provider in &snapshot.providers {
        lines.push(format!(
            "{:<TABLE_LABEL_WIDTH$} {:>12} {:>6.2}% {:>12} {:>12} {:>12}  {}",
            truncate_table_label(&provider.provider_id),
            format_exact_tokens(provider.total_tokens),
            provider.percentage,
            format_exact_tokens(provider.input_tokens),
            format_exact_tokens(provider.output_tokens),
            format_exact_tokens(provider.cache_tokens),
            format_price_summary(&provider.cost),
        ));
    }
    boxed_section("PROVIDER USAGE (EXACT TOKEN TOTALS)", &lines)
}

fn boxed_section(title: &str, lines: &[String]) -> String {
    let width = std::iter::once(title.len())
        .chain(lines.iter().map(String::len))
        .max()
        .unwrap_or_default();
    let border = format!("+{}+", "-".repeat(width + 2));
    let mut output = vec![
        border.clone(),
        format!("| {title:<width$} |"),
        border.clone(),
    ];
    output.extend(lines.iter().map(|line| format!("| {line:<width$} |")));
    output.push(border);
    output.join("\n")
}

fn truncate_table_label(value: &str) -> String {
    if value.chars().count() <= TABLE_LABEL_WIDTH {
        return value.to_string();
    }

    let mut label = value
        .chars()
        .take(TABLE_LABEL_WIDTH.saturating_sub(3))
        .collect::<String>();
    label.push_str("...");
    label
}

fn snapshot_buffer(text: &str) -> Buffer {
    let lines = text.lines().collect::<Vec<_>>();
    let width = lines.iter().map(|line| line.len()).max().unwrap_or(1);
    let mut buffer = Buffer::empty(Rect::new(
        0,
        0,
        u16::try_from(width + 2).unwrap_or(u16::MAX),
        u16::try_from(lines.len() + 3).unwrap_or(u16::MAX),
    ));

    for (index, line) in lines.iter().enumerate() {
        buffer.set_string(1, index as u16 + 1, line, Style::default());
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::{chart_dates, daily_bar};
    use crate::utils::time::TimeRange;
    use chrono::NaiveDate;

    #[test]
    fn daily_chart_uses_a_visible_bar_for_nonzero_values() {
        assert_eq!(daily_bar(0, 100), "");
        assert_eq!(daily_bar(1, 100).chars().count(), 1);
        assert_eq!(daily_bar(100, 100).chars().count(), 24);
    }

    #[test]
    fn limited_ranges_include_inactive_days() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();
        let dates = chart_dates(&[], TimeRange::Last7Days, today);

        assert_eq!(dates.len(), 7);
        assert_eq!(
            dates.first().copied(),
            Some(NaiveDate::from_ymd_opt(2026, 7, 25).unwrap())
        );
        assert_eq!(dates.last().copied(), Some(today));
    }
}
