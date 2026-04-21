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
    }

    Ok(())
}
