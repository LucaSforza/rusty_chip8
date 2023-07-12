use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct ChipArgs {
    /// the path to the rom
    pub path: String,
}
