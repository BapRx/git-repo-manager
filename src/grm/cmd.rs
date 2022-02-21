use clap::{AppSettings, Parser};

#[derive(Parser)]
#[clap(
    name = clap::crate_name!(),
    version = clap::crate_version!(),
    author = clap::crate_authors!("\n"),
    about = clap::crate_description!(),
    long_version = clap::crate_version!(),
    global_setting(AppSettings::DeriveDisplayOrder),
    propagate_version = true,
)]
pub struct Opts {
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    #[clap(about = "Manage repositories")]
    Repos(Repos),
    #[clap(visible_alias = "wt", about = "Manage worktrees")]
    Worktree(Worktree),
}

#[derive(Parser)]
pub struct Repos {
    #[clap(subcommand, name = "action")]
    pub action: ReposAction,
}

#[derive(Parser)]
pub enum ReposAction {
    #[clap(
        visible_alias = "run",
        about = "Synchronize the repositories to the configured values"
    )]
    Sync(Sync),
    #[clap(about = "Generate a repository configuration from an existing file tree")]
    Find(Find),
    #[clap(about = "Show status of configured repositories")]
    Status(OptionalConfig),
}

#[derive(Parser)]
#[clap()]
pub struct Sync {
    #[clap(
        short,
        long,
        default_value = "./config.toml",
        help = "Path to the configuration file"
    )]
    pub config: String,
}

#[derive(Parser)]
#[clap()]
pub struct OptionalConfig {
    #[clap(short, long, help = "Path to the configuration file")]
    pub config: Option<String>,
}

#[derive(clap::ArgEnum, Clone)]
pub enum ConfigFormat {
    Yaml,
    Toml,
}

#[derive(Parser)]
pub struct Find {
    #[clap(help = "The path to search through")]
    pub path: String,

    #[clap(
        arg_enum,
        short,
        long,
        help = "Format to produce",
        default_value_t = ConfigFormat::Toml,
    )]
    pub format: ConfigFormat,
}

#[derive(Parser)]
pub struct Worktree {
    #[clap(subcommand, name = "action")]
    pub action: WorktreeAction,
}

#[derive(Parser)]
pub enum WorktreeAction {
    #[clap(about = "Add a new worktree")]
    Add(WorktreeAddArgs),
    #[clap(about = "Add an existing worktree")]
    Delete(WorktreeDeleteArgs),
    #[clap(about = "Show state of existing worktrees")]
    Status(WorktreeStatusArgs),
    #[clap(about = "Convert a normal repository to a worktree setup")]
    Convert(WorktreeConvertArgs),
    #[clap(about = "Clean all worktrees that do not contain uncommited/unpushed changes")]
    Clean(WorktreeCleanArgs),
    #[clap(about = "Fetch refs from remotes")]
    Fetch(WorktreeFetchArgs),
    #[clap(about = "Fetch refs from remotes and update local branches")]
    Pull(WorktreePullArgs),
    #[clap(about = "Rebase worktree onto default branch")]
    Rebase(WorktreeRebaseArgs),
}

#[derive(Parser)]
pub struct WorktreeAddArgs {
    #[clap(help = "Name of the worktree")]
    pub name: String,

    #[clap(short = 't', long = "track", help = "Remote branch to track")]
    pub track: Option<String>,

    #[clap(long = "--no-track", help = "Disable tracking")]
    pub no_track: bool,
}
#[derive(Parser)]
pub struct WorktreeDeleteArgs {
    #[clap(help = "Name of the worktree")]
    pub name: String,

    #[clap(
        long = "force",
        help = "Force deletion, even when there are uncommitted/unpushed changes"
    )]
    pub force: bool,
}

#[derive(Parser)]
pub struct WorktreeStatusArgs {}

#[derive(Parser)]
pub struct WorktreeConvertArgs {}

#[derive(Parser)]
pub struct WorktreeCleanArgs {}

#[derive(Parser)]
pub struct WorktreeFetchArgs {}

#[derive(Parser)]
pub struct WorktreePullArgs {
    #[clap(long = "--rebase", help = "Perform a rebase instead of a fast-forward")]
    pub rebase: bool,
    #[clap(long = "--stash", help = "Stash & unstash changes before & after pull")]
    pub stash: bool,
}

#[derive(Parser)]
pub struct WorktreeRebaseArgs {
    #[clap(long = "--pull", help = "Perform a pull before rebasing")]
    pub pull: bool,
    #[clap(long = "--rebase", help = "Perform a rebase when doing a pull")]
    pub rebase: bool,
    #[clap(
        long = "--stash",
        help = "Stash & unstash changes before & after rebase"
    )]
    pub stash: bool,
}

pub fn parse() -> Opts {
    Opts::parse()
}
