use clap::Args;

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(required_unless_present("all"), value_delimiter = ',',help ="The name of the test case to remove. \nIf multiple test cases are specified(Use a comma between cases), all of them will be removed")]
    pub test_name: Option<Vec<String>>,

    #[arg(short, long, help="Remove all tests")]
    pub all: bool,
}
