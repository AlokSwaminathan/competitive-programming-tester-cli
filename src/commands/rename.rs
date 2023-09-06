
use clap::Args;

#[derive(Debug, Args)]
pub struct RenameArgs{
    #[arg(help="The name of the test case to rename")]
    pub(crate) test_name: String,
    #[arg(help="The new name of the test case")]
    pub(crate) new_name: String,
}
