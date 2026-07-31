mod analytics;
mod cache;
mod config;
mod db;
mod ui;
mod utils;

use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use color_eyre::eyre::{Context, ContextCompat, Result, bail};

use crate::cache::models_cache::{PricingCatalog, default_cache_path, refresh_pricing_catalog};
use crate::config::app_config::AppConfig;
use crate::config::theme_config::ThemeCatalog;
use crate::db::models::InputOptions;
use crate::db::queries::load_app_data;
use crate::ui::app::{App, print_exit_art};
use crate::ui::snapshot::{SnapshotFormat, SnapshotView, render_snapshot, write_snapshot_image};
use crate::ui::theme::{Theme, ThemeKind, ThemeMode};
use crate::utils::pricing::ZeroCostBehavior;
use crate::utils::time::TimeRange;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let CliArgs {
        command,
        database_path,
        json_path,
        theme,
        ignore_zero,
    } = CliArgs::parse();
    let input_options = InputOptions {
        database_path,
        json_path,
    };
    if let Some(command) = command {
        return run_command(command, &input_options, ignore_zero, theme).await;
    }

    let data = load_app_data(&input_options).wrap_err("failed to load OpenCode usage data")?;

    let pricing = PricingCatalog::load().wrap_err("failed to load pricing catalog")?;
    let (theme_kind, theme) = resolve_theme(theme).wrap_err("failed to resolve theme")?;
    let zero_cost_behavior = ZeroCostBehavior::from_ignore_zero(ignore_zero);
    let app = App::new(data, pricing, theme, zero_cost_behavior);
    app.run().await?;
    print_exit_art(theme_kind);
    Ok(())
}

#[derive(Debug, Parser)]
#[command(name = "shaw-oc-stats")]
#[command(version, about)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(
        long = "db",
        global = true,
        value_name = "PATH",
        help = "Path to OpenCode SQLite database file"
    )]
    database_path: Option<PathBuf>,

    #[arg(
        long = "json",
        global = true,
        value_name = "PATH",
        help = "Path to OpenCode usage JSON file"
    )]
    json_path: Option<PathBuf>,

    #[arg(
        long = "theme",
        global = true,
        help = "Theme to use for the application"
    )]
    theme: Option<ThemeMode>,

    #[arg(
        long = "ignore-zero",
        global = true,
        help = "Treat zero stored costs as missing and estimate them"
    )]
    ignore_zero: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Print a non-interactive usage snapshot to standard output")]
    Snapshot {
        #[arg(
            long,
            value_enum,
            default_value_t = SnapshotRange::All,
            help = "Time range to include: all, 7d, or 30d"
        )]
        range: SnapshotRange,

        #[arg(long, help = "Print only the per-day chart with exact token values")]
        daily: bool,

        #[arg(
            long,
            visible_alias = "models",
            help = "Print only usage grouped by model"
        )]
        model: bool,

        #[arg(long, help = "Print the complete snapshot (the default)")]
        all: bool,

        #[arg(
            long,
            value_enum,
            default_value_t = SnapshotFormat::Terminal,
            help = "Snapshot output format: terminal (ASCII art) or image (PNG)"
        )]
        format: SnapshotFormat,

        #[arg(
            long,
            value_name = "PATH",
            help = "Destination PNG path; required when --format image is used"
        )]
        output: Option<PathBuf>,
    },
    Cache {
        #[command(subcommand)]
        action: CacheCommand,
    },
    #[command(about = "Generate shell completions for shaw-oc-stats")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
#[command(about = "Manage the local cache of model pricing data")]
enum CacheCommand {
    #[command(about = "Show the path to the local pricing cache file")]
    Path,
    #[command(about = "Update the local pricing cache")]
    Update,
    #[command(about = "Clean the local pricing cache")]
    Clean,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, clap::ValueEnum)]
enum SnapshotRange {
    #[default]
    #[value(name = "all")]
    All,
    #[value(name = "7d", aliases = ["7-days", "last-7-days"])]
    Last7Days,
    #[value(name = "30d", aliases = ["30-days", "last-30-days"])]
    Last30Days,
}

impl From<SnapshotRange> for TimeRange {
    fn from(value: SnapshotRange) -> Self {
        match value {
            SnapshotRange::All => Self::All,
            SnapshotRange::Last7Days => Self::Last7Days,
            SnapshotRange::Last30Days => Self::Last30Days,
        }
    }
}

async fn run_command(
    command: Command,
    input_options: &InputOptions,
    ignore_zero: bool,
    cli_theme: Option<ThemeMode>,
) -> Result<()> {
    match command {
        Command::Snapshot {
            range,
            daily,
            model,
            all,
            format,
            output,
        } => {
            let selected_views = daily as usize + model as usize + all as usize;
            if selected_views > 1 {
                bail!("choose at most one of --daily, --model, or --all");
            }

            let image_output = match format {
                SnapshotFormat::Terminal => {
                    if output.is_some() {
                        bail!("--output can only be used with --format image");
                    }
                    None
                }
                SnapshotFormat::Image => {
                    let output = output.ok_or_else(|| {
                        color_eyre::eyre::eyre!("--output PATH is required with --format image")
                    })?;
                    if output.extension().and_then(|ext| ext.to_str()) != Some("png") {
                        bail!("snapshot images must use a .png output path");
                    }
                    Some(output)
                }
            };

            let view = if daily {
                SnapshotView::Daily
            } else if model {
                SnapshotView::Model
            } else {
                SnapshotView::All
            };
            let range = TimeRange::from(range);
            let data =
                load_app_data(input_options).wrap_err("failed to load OpenCode usage data")?;
            let pricing = PricingCatalog::load().wrap_err("failed to load pricing catalog")?;
            let snapshot = analytics::build_snapshot(
                &data,
                &pricing,
                range,
                ZeroCostBehavior::from_ignore_zero(ignore_zero),
            );
            match format {
                SnapshotFormat::Terminal => {
                    println!("{}", render_snapshot(&snapshot, range, view));
                }
                SnapshotFormat::Image => {
                    let output = image_output.expect("image output path was validated above");
                    let (_, theme) =
                        resolve_theme(cli_theme).wrap_err("failed to resolve image theme")?;
                    write_snapshot_image(&snapshot, range, view, &theme, &output)?;
                    println!("Wrote snapshot image to {}", output.display());
                }
            }
            Ok(())
        }
        Command::Cache { action } => match action {
            CacheCommand::Path => {
                println!("{}", default_cache_path()?.display());
                Ok(())
            }
            CacheCommand::Update => {
                println!("Updating pricing cache...");
                let path = default_cache_path()?;
                let current = PricingCatalog::load().ok();
                let message = finalize_cache_update(
                    &path,
                    current.as_ref(),
                    refresh_pricing_catalog(path.clone())
                        .await
                        .map_err(color_eyre::eyre::Error::from),
                )?;
                println!("{message}");
                Ok(())
            }
            CacheCommand::Clean => {
                let path = default_cache_path()?;
                if path.exists() {
                    std::fs::remove_file(&path)
                        .wrap_err_with(|| format!("failed to remove {}", path.display()))?;
                }
                println!("Cleaned {}", path.display());
                Ok(())
            }
        },
        Command::Completions { shell } => {
            let mut cmd = CliArgs::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
    }
}

fn finalize_cache_update(
    path: &std::path::Path,
    current: Option<&PricingCatalog>,
    result: Result<PricingCatalog>,
) -> Result<String> {
    match result {
        Ok(_) => Ok(format!("Updated {}", path.display())),
        Err(err) => {
            let fallback_hint = current
                .map(PricingCatalog::refresh_failure_hint)
                .unwrap_or("current pricing fallback status is unknown");
            Err(err.wrap_err(format!(
                "failed to update {}; {fallback_hint}",
                path.display()
            )))
        }
    }
}

fn resolve_theme(cli_theme: Option<ThemeMode>) -> Result<(ThemeKind, Theme)> {
    let app_config = AppConfig::load().wrap_err("failed to load config.toml")?;
    let catalog = ThemeCatalog::load().wrap_err("failed to load theme catalog")?;

    let mode = cli_theme.unwrap_or(app_config.theme.default);
    let kind = mode.resolve();
    let selected_name = match kind {
        ThemeKind::Dark => app_config.theme.dark.as_str(),
        ThemeKind::Light => app_config.theme.light.as_str(),
    };

    let selected = catalog.get(selected_name).wrap_err_with(|| {
        format!(
            "theme '{selected_name}' not found; available themes: {}",
            catalog.names().join(", ")
        )
    })?;

    if selected.kind != kind {
        bail!(
            "theme '{selected_name}' has type {:?}, expected {:?}",
            selected.kind,
            kind
        );
    }

    Ok((kind, selected.theme.clone()))
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use color_eyre::eyre::{Result, eyre};

    use super::{CliArgs, Command, SnapshotFormat, SnapshotRange, finalize_cache_update};
    use crate::cache::models_cache::{PricingAvailability, PricingCatalog};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn test_catalog(availability: PricingAvailability) -> PricingCatalog {
        PricingCatalog {
            models: BTreeMap::new(),
            cache_path: PathBuf::from("/tmp/models.json"),
            refresh_needed: false,
            availability,
            load_notice: None,
        }
    }

    #[test]
    fn cache_update_success_keeps_success_message() {
        let path = Path::new("/tmp/models.json");
        let result = finalize_cache_update(
            path,
            None,
            Ok::<PricingCatalog, _>(test_catalog(PricingAvailability::Cached)),
        )
        .unwrap();

        assert_eq!(result, "Updated /tmp/models.json");
    }

    #[test]
    fn cache_update_failure_returns_error_with_fallback_hint() {
        let path = Path::new("/tmp/models.json");
        let err = finalize_cache_update(
            path,
            Some(&test_catalog(PricingAvailability::OverridesOnly)),
            Err(eyre!("network down")),
        )
        .unwrap_err();

        let message = format!("{err:#}");
        assert!(message.contains("failed to update /tmp/models.json"));
        assert!(message.contains("using local pricing overrides only"));
    }

    #[test]
    fn cache_update_failure_without_catalog_still_returns_error() {
        let path = Path::new("/tmp/models.json");
        let result: Result<PricingCatalog> = Err(eyre!("network down"));
        let err = finalize_cache_update(path, None, result).unwrap_err();

        assert!(format!("{err:#}").contains("current pricing fallback status is unknown"));
    }

    #[test]
    fn parses_a_daily_seven_day_snapshot_with_input_after_the_subcommand() {
        let cli = CliArgs::try_parse_from([
            "shaw-oc-stats",
            "snapshot",
            "--json",
            "/tmp/export.json",
            "--range",
            "7d",
            "--daily",
        ])
        .unwrap();

        assert_eq!(cli.json_path, Some(PathBuf::from("/tmp/export.json")));
        assert!(matches!(
            cli.command,
            Some(Command::Snapshot {
                range: SnapshotRange::Last7Days,
                daily: true,
                model: false,
                all: false,
                format: SnapshotFormat::Terminal,
                output: None,
            })
        ));
    }

    #[test]
    fn parses_an_image_snapshot_destination() {
        let cli = CliArgs::try_parse_from([
            "shaw-oc-stats",
            "snapshot",
            "--format",
            "image",
            "--output",
            "/tmp/snapshot.png",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Snapshot {
                format: SnapshotFormat::Image,
                output: Some(path),
                ..
            }) if path == PathBuf::from("/tmp/snapshot.png")
        ));
    }
}
