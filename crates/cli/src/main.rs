mod commands;

use clap::Parser;

#[derive(Parser)]
#[command(name = "blaniel")]
#[command(about = "Blaniel NPC AI - Quick configuration tool for AI-powered NPCs")]
#[command(version)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = commands::run(cli.command).await {
        eprintln!("\x1b[31mError:\x1b[0m {}", e);
        std::process::exit(1);
    }
}
