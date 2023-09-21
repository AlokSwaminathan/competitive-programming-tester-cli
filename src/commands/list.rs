use std::collections::HashMap;

use clap::{Args, Subcommand};

use crate::{handle_option, test_data::Test, DEFAULT_FOLDER_NAME};

//list command just lists all test cases, sort by name
//list test command lists all test cases for a specific test, sort by test_case name, --show-input, --show-output, both true by default --cases to specify a test case or multiple test cases

#[derive(Args, Debug)]
pub struct ListArgs {
    #[command(subcommand)]
    pub command: Option<ListCommands>,

    #[arg(long, help = "Show input and output types, as well as file names(If applicable), for each test")]
    show_io: bool,
}

#[derive(Subcommand, Debug)]
pub enum ListCommands {
    #[command(about = "List all test case names, or all/some test cases for a specific test")]
    TEST(ListTestArgs),
}

#[derive(Args, Debug)]
pub struct ListTestArgs {
    #[arg(help = "The name of the test to list cases for")]
    test: String,

    #[arg(short = 'i', long, help = "Show input for each test case(Input can be very large)")]
    show_input: bool,

    #[arg(short = 'o', long, help = "Show desired output for each test case")]
    show_output: bool,

    #[arg(
        short,
        long,
        requires = "test",
        value_delimiter = ',',
        help = "The name of the test case to list. \nIf multiple test cases are specified(Use a comma between cases), all of them will be listed. \nIf not specified, all test cases will be listed"
    )]
    cases: Option<Vec<String>>,
}

impl ListArgs {
    pub fn run(&self, tests: &mut HashMap<String, Test>) -> Result<(), String> {
        if tests.is_empty() {
            return Err("There are no tests to list".to_string());
        }
        match &self.command {
            Some(ListCommands::TEST(args)) => {
                let test = match tests.get_mut(&args.test) {
                    Some(test) => test,
                    None => return Err(format!("Test with name \"{}\" does not exist", &args.test)),
                };
                let data_dir = handle_option!(
                    dirs::data_local_dir(),
                    "Failed to get data directory, not sure why this should happen, look into dirs::data_local_dir() to find more about error"
                );
                let test_dir = data_dir.join(DEFAULT_FOLDER_NAME).join("tests").join(&args.test);
                test.fill_cases(test_dir)?;
                args.print_info(test)?;
                Ok(())
            }
            None => {
                println!("Tests:");
                let mut test_print_data: Vec<String> = tests
                    .iter()
                    .map(|(name, test)| {
                        let mut print_data = name.clone();
                        if self.show_io {
                            let (input_type, output_type) = test.get_io_types();
                            print_data += &format!("\n\tInput: {}, Output: {}", input_type, output_type);
                        }
                        print_data
                    })
                    .collect();
                test_print_data.sort();
                println!("{}", test_print_data.join("\n"));
                Ok(())
            }
        }
    }
}

impl ListTestArgs {
    fn print_info(&self, test: &Test) -> Result<(), String> {
        let all_case_names = test.get_sorted_case_names();
        let mut wanted_names = vec![];
        if let Some(cases) = &self.cases {
            for case in cases {
                wanted_names.push(case);
            }
        } else {
            wanted_names = all_case_names;
        }
        println!("Test cases:");

        for case_name in wanted_names {
            test.print_case(case_name, self.show_input, self.show_output)?;
        }
        Ok(())
    }
}
