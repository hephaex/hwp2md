use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hwp2md")]
#[command(about = "HWP/HWPX ↔ Markdown bidirectional converter")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert HWP/HWPX to Markdown
    ToMd {
        /// Input HWP or HWPX file
        input: PathBuf,
        /// Output Markdown file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Extract images to this directory
        #[arg(long)]
        assets_dir: Option<PathBuf>,
        /// Include frontmatter metadata
        #[arg(long)]
        frontmatter: bool,
    },
    /// Convert Markdown to HWPX
    ToHwpx {
        /// Input Markdown file
        input: PathBuf,
        /// Output HWPX file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Style template (YAML)
        #[arg(long)]
        style: Option<PathBuf>,
    },
    /// Show document info without converting
    Info {
        /// Input HWP or HWPX file
        input: PathBuf,
    },
    /// Validate a file by parsing it without writing output
    Check {
        /// Input file (.hwp, .hwpx, .md, or .markdown)
        input: PathBuf,
    },
    /// Convert between supported formats by inferring the direction from
    /// the input and output file extensions.
    ///
    /// Supported pairs:
    ///   .hwp / .hwpx  →  .md / .markdown
    ///   .md / .markdown  →  .hwpx
    Convert {
        /// Input file (.hwp, .hwpx, .md, or .markdown)
        input: PathBuf,
        /// Output file (.md, .markdown, or .hwpx)
        output: PathBuf,
        /// Overwrite the output file if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Batch-convert all HWP/HWPX files in a directory to Markdown
    Batch {
        /// Input directory containing .hwp/.hwpx files
        input_dir: PathBuf,
        /// Output directory for .md files (default: same as input)
        #[arg(short, long)]
        output_dir: Option<PathBuf>,
        /// Include frontmatter metadata
        #[arg(long)]
        frontmatter: bool,
        /// Overwrite existing output files
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    match cli.command {
        Commands::ToMd {
            input,
            output,
            assets_dir,
            frontmatter,
        } => {
            hwp2md::convert::to_markdown(
                &input,
                output.as_deref(),
                assets_dir.as_deref(),
                frontmatter,
            )?;
        }
        Commands::ToHwpx {
            input,
            output,
            style,
        } => {
            hwp2md::convert::to_hwpx(&input, output.as_deref(), style.as_deref())?;
        }
        Commands::Info { input } => {
            hwp2md::convert::show_info(&input)?;
        }
        Commands::Check { input } => {
            hwp2md::convert::check(&input)?;
            println!("OK: {}", input.display());
        }
        Commands::Convert {
            input,
            output,
            force,
        } => {
            hwp2md::convert::convert_auto(&input, &output, force)?;
        }
        Commands::Batch {
            input_dir,
            output_dir,
            frontmatter,
            force,
        } => {
            run_batch(&input_dir, output_dir.as_deref(), frontmatter, force)?;
        }
    }

    Ok(())
}

/// Batch-convert all `.hwp` / `.hwpx` files in `input_dir` to Markdown.
///
/// Each converted file is placed in `output_dir` (or `input_dir` when not
/// supplied) with the same stem and a `.md` extension.  Conversion errors for
/// individual files are logged and counted; the function returns `Ok(())` as
/// long as at least one file succeeded (or zero files were found).  It only
/// returns an `Err` when the input directory cannot be read or all files
/// failed.
fn run_batch(
    input_dir: &std::path::Path,
    output_dir: Option<&std::path::Path>,
    frontmatter: bool,
    force: bool,
) -> Result<()> {
    if !input_dir.exists() {
        anyhow::bail!(
            "input directory does not exist: {}",
            input_dir.display()
        );
    }
    if !input_dir.is_dir() {
        anyhow::bail!(
            "input path is not a directory: {}",
            input_dir.display()
        );
    }

    let out_dir = output_dir.unwrap_or(input_dir);
    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir)?;
    }

    let entries = std::fs::read_dir(input_dir)?;

    let mut converted: usize = 0;
    let mut skipped: usize = 0;
    let mut failed: usize = 0;

    for entry in entries {
        let entry = entry?;

        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        if entry.file_type()?.is_symlink() {
            continue;
        }

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "hwp" && ext != "hwpx" {
            continue;
        }

        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_owned(),
            None => {
                tracing::warn!("Skipping file with non-UTF-8 stem: {:?}", path);
                failed += 1;
                continue;
            }
        };

        let out_path = out_dir.join(format!("{stem}.md"));

        if !force && out_path.exists() {
            eprintln!(
                "Skipping {:?}: output already exists (use --force to overwrite)",
                path.display()
            );
            skipped += 1;
            continue;
        }

        match hwp2md::convert::to_markdown(&path, Some(&out_path), None, frontmatter) {
            Ok(()) => {
                println!("Converted: {} -> {}", path.display(), out_path.display());
                converted += 1;
            }
            Err(e) => {
                eprintln!("Error converting {}: {e}", path.display());
                failed += 1;
            }
        }
    }

    println!("Batch complete: {converted} converted, {skipped} skipped, {failed} failed");

    if converted == 0 && failed > 0 {
        anyhow::bail!("All {failed} file(s) failed to convert");
    }

    Ok(())
}
