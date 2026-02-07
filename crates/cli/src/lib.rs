use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use pdf_engine::{default_engine, OpenSource, PdfEngine, ThumbnailSize};
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Parser)]
#[command(name = "butterpaper-cli")]
#[command(about = "ButterPaper CLI")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Open a PDF in the desktop app.
    Open {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Print machine-readable PDF metadata.
    Info {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Render a thumbnail PNG for a page.
    RenderThumb {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 320)]
        width: u32,
        #[arg(long, default_value_t = 320)]
        height: u32,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Print CLI version.
    Version,
}

#[derive(Debug, Serialize)]
struct InfoOutput {
    path: String,
    page_count: u32,
    first_page_size_pt: Option<PageSizeOutput>,
}

#[derive(Debug, Serialize)]
struct PageSizeOutput {
    width: f32,
    height: f32,
}

pub fn run<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Open { file } => run_open(&file),
        Commands::Info { file } => run_info(&file),
        Commands::RenderThumb { file, page, width, height, output } => {
            run_render_thumb(&file, page, width, height, output.as_deref())
        }
        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn run_open(file: &Path) -> Result<()> {
    ensure_pdf_exists(file)?;

    if std::env::var_os("BUTTERPAPER_TEST_NO_SPAWN").is_some() {
        println!("open:{}", file.display());
        return Ok(());
    }

    let desktop_bin = std::env::var_os("BUTTERPAPER_APP_BIN")
        .or_else(|| std::env::var_os("BUTTERPAPER_DESKTOP_BIN"))
        .unwrap_or_else(|| OsString::from("butterpaper"));

    let status =
        Command::new(desktop_bin).arg(file).status().context("failed to launch desktop app")?;

    if !status.success() {
        anyhow::bail!("desktop app exited with status {status}");
    }

    Ok(())
}

fn run_info(file: &Path) -> Result<()> {
    ensure_pdf_exists(file)?;

    let mut engine = default_engine();
    let handle = engine.open(OpenSource::from(file)).context("failed to open PDF")?;

    let page_count = engine.page_count(handle)?;
    let first_page_size_pt = if page_count > 0 {
        let size = engine.page_size(handle, 0)?;
        Some(PageSizeOutput { width: size.width_pt, height: size.height_pt })
    } else {
        None
    };

    let payload = InfoOutput { path: file.display().to_string(), page_count, first_page_size_pt };

    let json = serde_json::to_string_pretty(&payload)?;
    println!("{json}");

    engine.close(handle)?;

    Ok(())
}

fn run_render_thumb(
    file: &Path,
    page: u32,
    width: u32,
    height: u32,
    output: Option<&Path>,
) -> Result<()> {
    ensure_pdf_exists(file)?;

    if page == 0 {
        anyhow::bail!("--page is 1-based and must be >= 1");
    }

    let mut engine = default_engine();
    let handle = engine.open(OpenSource::from(file)).context("failed to open PDF")?;

    let page_index = page - 1;
    let image = engine
        .render_thumbnail(handle, page_index, ThumbnailSize { width_px: width, height_px: height })
        .context("failed to render thumbnail")?;

    let output =
        output.map(ToOwned::to_owned).unwrap_or_else(|| default_thumbnail_output(file, page));

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    image
        .save(&output)
        .with_context(|| format!("failed to write image to {}", output.display()))?;

    println!("{}", output.display());

    engine.close(handle)?;

    Ok(())
}

fn ensure_pdf_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("file does not exist: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("path is not a file: {}", path.display());
    }

    Ok(())
}

fn default_thumbnail_output(file: &Path, page: u32) -> PathBuf {
    let stem = file.file_stem().and_then(|name| name.to_str()).unwrap_or("thumbnail");

    file.with_file_name(format!("{stem}-page-{page}.png"))
}
