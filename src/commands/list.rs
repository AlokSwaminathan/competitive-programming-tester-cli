use std::collections::HashMap;

use clap::{Args, Subcommand};
use tabled::{
    settings::{locator::ByColumnName, Disable},
    Table, Tabled,
};

use crate::{handle_option, test_data::Test, DEFAULT_FOLDER_NAME};

//list command just lists all test cases, sort by name
//list test command lists all test cases for a specific test, sort by test_case name, --show-input, --show-output, both true by default --cases to specify a test case or multiple test cases

#[derive(Args, Debug)]
pub struct ListArgs {
    #[command(subcommand)]
    pub command: Option<ListCommands>,

    #[arg(long, help = "Show input and output types, as well as file names(If applicable), for each test")]
    show_io: bool,

    #[arg(
        short,
        long,
        help = "Pass a submission type (usaco, codeforces, or atcoder) and only tests with that submisison type will be listed"
    )]
    submission_type: Option<String>,
}

#[derive(Tabled, Debug)]
struct TestTable {
    #[tabled(rename = "Test Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
    #[tabled(rename = "Submission Type")]
    submission_type: String,
    #[tabled(rename = "Input Type")]
    input_type: String,
    #[tabled(rename = "Output Type")]
    output_type: String,
}

#[derive(Tabled, Debug)]
struct CaseTable<'a> {
    #[tabled(rename = "Case Name")]
    case_name: String,
    #[tabled(rename = "Input File(In Test Folder)")]
    input_file: String,
    #[tabled(rename = "Output File(In Test Folder)")]
    output_file: String,
    #[tabled(rename = "Input")]
    input: &'a str,
    #[tabled(rename = "Output")]
    output: &'a str,
}

impl TestTable {
    pub fn from_tests(tests: &HashMap<String, Test>, submission_type: &Option<String>) -> Vec<TestTable> {
        let mut table_data = vec![];
        for (name, test) in tests {
            if let Some(submission_type) = submission_type {
                if submission_type != &test.get_submission_type() {
                    continue;
                }
            }
            let (input_type, output_type) = test.get_io_types();
            table_data.push(TestTable {
                name: name.clone(),
                description: test.description.as_ref().unwrap_or(&"None".to_string()).clone(),
                submission_type: {
                    match &test.submission_data {
                        Some(submission_data) => format!("{}", submission_data.submission_type),
                        None => "None".to_string(),
                    }
                },
                input_type,
                output_type,
            });
        }
        table_data.sort_by_key(|x| x.name.clone());
        table_data
    }
}

impl<'b> CaseTable<'_> {
    pub fn from_test<'a>(test: &'a Test, case_names: &Vec<String>) -> Result<Vec<CaseTable<'a>>,String> {
        let all_cases = test.get_sorted_case_names();
        let mut table_data = vec![];
        let mut temp_case_names = vec![];
        if case_names.is_empty() {
            temp_case_names.extend(&all_cases)
        } else {
            for case in case_names {
                temp_case_names.push(case);
            }
        };
        let case_names = temp_case_names;
        for case_name in case_names {
            if !all_cases.contains(&case_name) {
                return Err(format!("Test case with name \"{}\" does not exist", case_name));
            }
            table_data.push(CaseTable {
                case_name: case_name.clone(),
                input_file: format!("{}.{}", case_name, test.input_extension),
                output_file: format!("{}.{}", case_name, test.output_extension),
                input: &test.cases.get(case_name).unwrap().input,
                output: &test.cases.get(case_name).unwrap().output
            });
        }
        Ok(table_data)
    }
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
                let case_tables = CaseTable::from_test(test, args.cases.as_ref().unwrap_or(&vec![]))?;
                let mut case_table = Table::new(case_tables);
                if !args.show_input {
                    case_table.with(Disable::column(ByColumnName::new("Input")));
                }
                if !args.show_output {
                    case_table.with(Disable::column(ByColumnName::new("Output")));
                }
                println!("{case_table}");
                Ok(())
            }
            None => {
                let test_tables = TestTable::from_tests(tests, &self.submission_type);
                let test_table = Table::new(test_tables);
                println!("{test_table}");
                Ok(())
            }
        }
    }
}