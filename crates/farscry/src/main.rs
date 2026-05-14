mod clipboard;
mod commands;
mod config;
mod pipeline;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "farscry")]
#[command(version = "0.1.0")]
#[command(about = "Visual automation workflow Protocol CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(required = false)]
        paths: Vec<PathBuf>,

        #[arg(long)]
        from_clipboard: bool,

        #[arg(long)]
        json: bool,

        #[arg(long)]
        affordances: bool,

        #[arg(long)]
        text_only: bool,

        #[arg(long)]
        context: bool,

        #[arg(long, default_value = "eng")]
        lang: String,

        #[arg(long, default_value = "10")]
        max_size: u64,

        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    Diff {
        before: PathBuf,
        after: PathBuf,

        #[arg(long)]
        json: bool,
    },

    Serve {
        #[arg(long)]
        mcp: bool,

        #[arg(long)]
        port: Option<u16>,
    },

    InstallLang {
        #[arg(required = true)]
        lang: Vec<String>,
    },

    Setup {
        #[arg(long)]
        undo_smart_paste: bool,
    },

    Paste {
        #[arg(long)]
        agent: Option<String>,

        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },

    Annotate {
        #[arg(required = false)]
        paths: Vec<PathBuf>,

        #[arg(long)]
        from_clipboard: bool,

        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: Result<()> = match cli.command {
        Commands::Extract {
            paths,
            from_clipboard,
            json,
            affordances,
            text_only,
            context,
            lang,
            max_size,
            output,
        } => {
            let max_size_bytes = max_size * 1024 * 1024;
            let opts = commands::extract::ExtractOpts {
                json,
                affordances,
                text_only,
                context,
                output,
            };
            if from_clipboard {
                commands::extract::extract_from_clipboard(opts, &lang, max_size_bytes)
            } else if paths.is_empty() {
                commands::extract::extract_from_stdin(opts, &lang, max_size_bytes)
            } else {
                commands::extract::extract_images(paths, opts, &lang, max_size_bytes)
            }
        }
        Commands::Diff {
            before,
            after,
            json,
        } => commands::diff::diff_images(before, after, json),
        Commands::Serve { mcp, port } => commands::serve::serve_mcp(mcp, port).await,
        Commands::InstallLang { lang } => commands::install::install_lang(lang),
        Commands::Setup { undo_smart_paste } => {
            if undo_smart_paste {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                commands::setup::undo_smart_paste_configs(&home)
            } else {
                commands::setup::setup()
            }
        }
        Commands::Paste { agent, prompt } => {
            let prompt_str = if prompt.is_empty() {
                None
            } else {
                Some(prompt.join(" "))
            };
            commands::paste::paste(agent.as_deref(), prompt_str.as_deref())
        }
        Commands::Annotate {
            paths,
            from_clipboard,
            output,
        } => {
            if from_clipboard {
                commands::annotate::annotate_from_clipboard(output)
            } else {
                commands::annotate::annotate_images(paths, output)
            }
        }
    };

    match result {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            let exit_code = if e.to_string().contains("file not found")
                || e.to_string().contains("invalid input")
                || e.to_string().contains("not an image")
            {
                1
            } else if e.to_string().contains("OCR failed") || e.to_string().contains("model error")
            {
                2
            } else if e.to_string().contains("language not installed")
                || e.to_string().contains("configuration")
            {
                3
            } else {
                1
            };
            process::exit(exit_code);
        }
    }
}
