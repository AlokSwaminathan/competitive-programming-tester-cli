use crate::commands::add::SubmissionData;
use crate::{handle_error, handle_option};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Test {
    cases: HashMap<String, TestCase>,
    input_extension: String,
    output_extension: String,
    input_io: IOType,
    output_io: IOType,
    submission_type: Option<SubmissionData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyTest {
    input_extension: String,
    output_extension: String,
    input_io: IOType,
    output_io: IOType,
    submission_type: Option<SubmissionData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestCase {
    input: String,
    output: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IOType {
    STD,
    FILE(PathBuf),
}

impl Test {
    pub fn print_case(&self, case_name: &String, show_input: bool, show_output: bool) -> Result<(), String> {
        let test_case = handle_option!(self.cases.get(case_name), format!("Test case with name \"{}\" does not exist", case_name));
        if show_input || show_output {
            println!("Test Case {}:", case_name);
            if show_input {
                println!("\tInput({}.{}):", case_name, &self.input_extension);
                println!(
                    "{}",
                    test_case.input.lines().map(|l| format!("\t\t{}", l)).collect::<Vec<String>>().join("\n")
                );
            }
            if show_output {
                println!("\tOutput({}.{}):", case_name, &self.output_extension);
                println!(
                    "{}",
                    test_case.output.lines().map(|l| format!("\t\t{}", l)).collect::<Vec<String>>().join("\n")
                );
            }
        } else {
            println!(
                "Name: {0} (Input: {0}.{1}, Output: {0}.{2})",
                case_name, &self.input_extension, &self.output_extension
            );
        }
        Ok(())
    }

    pub fn get_sorted_case_names(&self) -> Vec<&String> {
        let mut case_names = self.cases.keys().collect::<Vec<&String>>();
        let mut case_int: Vec<Option<i32>> = case_names.iter().map(|c| c.parse::<i32>().ok()).collect();
        let all_parsed = case_int.iter().all(|c| c.is_some());
        if all_parsed {
            case_int.sort();
            case_names.sort_by_key(|c| c.parse::<i32>().unwrap());
        } else {
            case_names.sort();
        }

        case_names
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    pub fn from_folder(folder: PathBuf, input_type: String, output_type: String, input_io: IOType, output_io: IOType, submission_type: Option<SubmissionData>) -> Result<Test, String> {
        let mut test = Test {
            cases: HashMap::new(),
            input_extension: input_type,
            output_extension: output_type,
            input_io,
            output_io,
            submission_type
        };
        test.fill_cases(folder)?;

        Ok(test)
    }
    pub fn fill_cases(&mut self, folder: PathBuf) -> Result<(), String> {
        let files = handle_error!(folder.read_dir(), "Invalid folder, can't read directory");
        let mut test_case_files: Vec<(PathBuf, PathBuf)> = Vec::new();
        let files = files
            .filter_map(|file| {
                if file.is_err() {
                    println!("Invalid file in folder: {}", file.as_ref().err().unwrap().to_string());
                }

                file.ok()
            })
            .collect::<Vec<DirEntry>>();
        for file in files {
            let file_path = file.path();
            let extension = file_path.extension();
            if let Some(extension) = extension {
                let extension = handle_option!(extension.to_str(), format!("Invalid file extension, {:?}, is not valid utf-8", extension));
                if extension == &self.input_extension {
                    let file_name = file_path.file_stem();
                    if let Some(file_name) = file_name {
                        let mut output_path = folder.join(PathBuf::from(file_name));
                        output_path.set_extension(&self.output_extension);
                        if output_path.exists() {
                            test_case_files.push((file_path, output_path));
                        }
                    }
                }
            }
        }
        if test_case_files.is_empty() {
            return Err(format!(
                "No test cases found(Input extension is \".{}\", Output extension is \".{}\")",
                &self.input_extension, &self.output_extension
            ));
        }
        for file_set in test_case_files {
            let name = &file_set.0.file_stem();
            let name = handle_option!(name, "Invalid file, can't get file name, this error shouldn't happen");
            let name = handle_option!(name.to_str(), "Invalid file name, is not valid utf-8, this error shouldn't happen");
            let name = name.to_string();
            let input_data = handle_error!(fs::read(file_set.0), "Invalid input file, can't read file");
            let output_data = handle_error!(fs::read(file_set.1), "Invalid output file, can't read file");
            let test_case = TestCase::new(input_data, output_data)?;
            self.cases.insert(name, test_case);
        }
        Ok(())
    }

    pub fn write_data(&self, path: &PathBuf) -> Result<(), String> {
        for (name, test_case) in &self.cases {
            let input_file = format!("{}.{}", name, self.input_extension);
            let output_file = format!("{}.{}", name, self.output_extension);
            let input_path = path.join(PathBuf::from(input_file));
            let output_path = path.join(PathBuf::from(output_file));
            test_case.write_data(&input_path, &output_path, name)?;
        }

        Ok(())
    }

    pub fn set_cases(&mut self, cases: &Option<Vec<String>>) -> Result<(), String> {
        if let Some(cases) = cases {
            let mut new_cases = HashMap::new();
            for case in cases {
                if let Some(test_case) = self.cases.get(case) {
                    new_cases.insert(case.clone(), test_case.clone());
                } else {
                    return Err(format!("Test case with name \"{}\" does not exist", case));
                }
            }
            self.cases = new_cases;
        }
        Ok(())
    }
    pub fn get_files(&self, temp_path: &PathBuf) -> (Option<PathBuf>, Option<PathBuf>) {
        let input_file = match &self.input_io {
            IOType::STD => None,
            IOType::FILE(path) => Some(temp_path.join(path)),
        };
        let output_file = match &self.output_io {
            IOType::STD => None,
            IOType::FILE(path) => Some(temp_path.join(path)),
        };
        (input_file, output_file)
    }
    pub fn case_iter(&self) -> impl Iterator<Item = (&String, &TestCase)> {
        let sorted_names = self.get_sorted_case_names();
        let sorted_vec: Vec<(&String, &TestCase)> = sorted_names.iter().map(|name| (*name, self.cases.get(*name).unwrap())).collect();
        sorted_vec.into_iter()
    }
    pub fn get_io_types(&self) -> (String, String) {
        (self.input_io.to_string(true), self.output_io.to_string(false))
    }
}

impl TestCase {
    pub fn new(input_data: Vec<u8>, output_data: Vec<u8>) -> Result<TestCase, String> {
        let input = handle_error!(String::from_utf8(input_data), "Invalid input data for a test case");
        let output = handle_error!(String::from_utf8(output_data), "Invalid output data for a test case");
        Ok(TestCase { input, output })
    }

    pub fn write_data(&self, input_path: &PathBuf, output_path: &PathBuf, name: &String) -> Result<(), String> {
        self.write_input(input_path, name)?;
        self.write_output(output_path, name)?;
        Ok(())
    }
    pub fn write_input(&self, input_path: &PathBuf, name: &String) -> Result<(), String> {
        handle_error!(
            fs::write(input_path, &self.input),
            format!("Failed to write test case input to file({:?}) for test case \"{}\"", input_path, name)
        );
        Ok(())
    }
    pub fn write_output(&self, output_path: &PathBuf, name: &String) -> Result<(), String> {
        handle_error!(
            fs::write(output_path, &self.output),
            format!("Failed to write test case output to file({:?}) for test case \"{}\"", output_path, name)
        );
        Ok(())
    }
    pub fn get_input(&self) -> &String {
        &self.input
    }
    pub fn get_output(&self) -> &String {
        &self.output
    }
}

impl From<EmptyTest> for Test {
    fn from(empty_test: EmptyTest) -> Self {
        Test {
            cases: HashMap::new(),
            input_extension: empty_test.input_extension,
            output_extension: empty_test.output_extension,
            input_io: empty_test.input_io,
            output_io: empty_test.output_io,
            submission_type: empty_test.submission_type
        }
    }
}

impl From<&Test> for EmptyTest {
    fn from(test: &Test) -> Self {
        EmptyTest {
            input_extension: test.input_extension.clone(),
            output_extension: test.output_extension.clone(),
            input_io: test.input_io.clone(),
            output_io: test.output_io.clone(),
            submission_type: test.submission_type.clone()
        }
    }
}

impl IOType {
    pub fn to_string(&self, input: bool) -> String {
        match self {
            IOType::STD => if input { "stdin" } else { "stdout" }.to_string(),
            IOType::FILE(path) => path.to_string_lossy().to_string(),
        }
    }
}
