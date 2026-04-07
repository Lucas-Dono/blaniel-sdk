pub mod init;
pub mod npc;
pub mod llm;
pub mod scene;
pub mod actions;
pub mod setup;

use blaniel_sdk::SdkError;

#[derive(clap::Subcommand)]
pub enum Commands {
    Init(init::InitCommand),
    #[command(subcommand)]
    Npc(npc::NpcCommands),
    #[command(subcommand)]
    Llm(llm::LlmCommands),
    #[command(subcommand)]
    Scene(scene::SceneCommands),
    #[command(subcommand)]
    Actions(actions::ActionCommands),
    Setup(setup::SetupCommand),
}

pub async fn run(cmd: Commands) -> Result<(), SdkError> {
    match cmd {
        Commands::Init(c) => c.run().await,
        Commands::Npc(c) => npc::run(c).await,
        Commands::Llm(c) => llm::run(c).await,
        Commands::Scene(c) => scene::run(c).await,
        Commands::Actions(c) => actions::run(c).await,
        Commands::Setup(c) => c.run().await,
    }
}
