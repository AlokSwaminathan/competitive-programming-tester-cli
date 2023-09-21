use std::process::exit;

mod commands {
    pub mod add;
    pub mod config;
    pub mod list;
    pub mod remove;
    pub mod rename;
    pub mod run;
}
mod cli;
mod config;
mod macros;
mod program_data;
mod test_data;
use program_data::ProgramData;

// Implementation ideas
// Download test data and store it
// Run test on a file
// Supports all languages USACO supports
// Configuration for the compilation of the files and the names of the executables
// tester add <link>
// tester remove <test>
// tester run <test> (-f <file>) (-b binary)
// tester list
// tester config

const DEFAULT_FOLDER_NAME: &str = "usaco-tester";

fn main() {
    let mut program_data = ProgramData::new();

    let program_result = program_data.run();

    let file_clear_result = program_data.clear_temp_files();
    if let Err(e) = file_clear_result {
        eprintln!("Failed to clear temporary files: {}", e);
    }

    match program_result {
        Err(e) => {
            eprintln!("\x1b[31mERROR\x1b[0m: {e}");
            exit(1)
        }
        _ => (),
    };
}
