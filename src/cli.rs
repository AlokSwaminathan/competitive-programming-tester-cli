use crate::commands::{add, list, remove, run, rename, config};
use std::fmt::Debug;

#[allow(unused_imports)]
use clap::{error::ErrorKind, Args, CommandFactory, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "CPTester",
    version = "0",
    author = "Alok Swaminathan <swaminathanalok@gmail.com>",
    arg_required_else_help = true,
    about = "A simple command line tool that can be used to easily add tests for Competitive Programming problems and run them.\nSupports C, C++, Java, and Python, but Java and Python use the versions installed on your system and C uses the default version.\nJava files name should be the same as the class name",
)]
pub struct CliData {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
#[allow(non_camel_case_types)]
pub enum Commands {
    #[command(about="Add a test case",arg_required_else_help=true)]
    ADD(add::AddArgs),
    #[command(about="Remove a test case",arg_required_else_help=true)]
    REMOVE(remove::RemoveArgs),
    #[command(about="List tests, test cases, or test info",arg_required_else_help=true)]
    LIST(list::ListArgs),
    #[command(about="Run a test case, supports C, C++, Java, and Python. Java and Python use the versions installed on your system",arg_required_else_help=true)]
    RUN(run::RunArgs),
    #[command(about="Rename a test case",arg_required_else_help=true)]
    RENAME(rename::RenameArgs),
    #[command(about="Work with the config of the program",arg_required_else_help=true)]
    CONFIG(config::ConfigArgs),
}

