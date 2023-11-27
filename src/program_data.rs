use crate::commands::run::RunDir;
use crate::config::Config;
use crate::{
    cli::{CliData, Commands},
    handle_option,
    test_data::{EmptyTest, Test},
};
use crate::{handle_error, DEFAULT_FOLDER_NAME};
use clap::Parser;
use std::fs;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct ProgramData {
    cli_data: CliData,
    tests: HashMap<String, Test>,
    pub temp_path: Option<PathBuf>,
}

impl ProgramData {
    pub fn new() -> ProgramData {
        let cli = CliData::parse();
        ProgramData {
            cli_data: cli,
            tests: HashMap::new(),
            temp_path: None,
        }
    }
    pub fn run(&mut self) -> Result<(), String> {
        let tests = handle_error!(ProgramData::load_empty_tests(), "Failed to load empty(Without input & output data) tests");
        self.tests = tests;
        match &self.cli_data.command {
            Some(Commands::ADD(args)) => {
                let (input_io, output_io) = handle_error!(args.get_io(), "Failed to get IO Data");
                let (test_name, test_path, submission_data) = handle_error!(args.get_test_data(), "Failed to get test data");
                if !args.input_type_is_folder() {
                    self.temp_path = Some(test_path.clone());
                }
                let test = handle_error!(
                    Test::from_folder(
                        test_path,
                        args.input_extension.clone(),
                        args.output_extension.clone(),
                        input_io,
                        output_io,
                        submission_data,
                    ),
                    "Failed to create test from folder/zip"
                );
                self.tests.insert(test_name, test);
                handle_error!(self.write_data(), "Failed to write data for new test");
                Ok(())
            }
            Some(Commands::LIST(args)) => Ok(handle_error!(args.run(&mut self.tests), "Failed to list test/cases")),
            Some(Commands::REMOVE(args)) => {
                if args.all {
                    if self.tests.is_empty() {
                        return Err("There are no tests to remove".to_string());
                    }
                    self.tests.clear();
                    let test_path = handle_option!(dirs::data_local_dir(), "Failed to get data local dir, dirs crate issue");
                    let test_path = test_path.join(DEFAULT_FOLDER_NAME).join("tests");
                    handle_error!(fs::remove_dir_all(test_path), "Failed to remove test directory");
                    println!("Successfully removed all tests");
                    return self.write_data();
                }
                let test_names = args.test_name.as_ref().unwrap();
                for test_name in test_names {
                    if let Some(_) = self.tests.remove_entry(test_name) {
                        let test_path = handle_option!(dirs::data_local_dir(), "Failed to get data local dir, dirs crate issue");
                        let test_path = test_path.join(DEFAULT_FOLDER_NAME).join("tests").join(test_name);
                        handle_error!(fs::remove_dir_all(test_path), "Failed to remove test directory");
                        println!("Successfully removed test with name \"{}\" ", test_name);
                    } else {
                        return Err(format!("Test with name \"{}\" doesn't exist", test_name));
                    }
                }

                self.write_data()
            }
            Some(Commands::RUN(args)) => {
                let test_name = &args.test;
                if !self.tests.contains_key(test_name) {
                    return Err(format!("Test with name \"{}\" doesn't exist", test_name));
                };
                let config = handle_error!(Config::get(), "Failed to load in config");
                let test = self.tests.get_mut(test_name).unwrap();
                let folder = handle_option!(dirs::data_local_dir(), "Failed to get data local dir, dirs crate issue");
                let folder = folder.join(DEFAULT_FOLDER_NAME).join("tests").join(test_name);
                handle_error!(test.fill_cases(folder), "Failed to get config");
                let mut run_dir = handle_error!(RunDir::new(test, &args, &config), "Failed to compile file and store in temp dir");
                handle_error!(run_dir.run(), "Failed to run test");
                Ok(())
            }
            Some(Commands::RENAME(args)) => {
                let old_name = &args.test_name;
                let new_name = &args.new_name;
                if !self.tests.contains_key(old_name) {
                    return Err(format!("Test with name \"{}\" doesn't exist", old_name));
                }
                if self.tests.contains_key(new_name) {
                    return Err(format!("Test with name \"{}\" already exists", new_name));
                }
                let (_, test) = self.tests.remove_entry(old_name).unwrap();
                self.tests.insert(new_name.clone(), test);
                let data_dir = handle_option!(
                    dirs::data_local_dir(),
                    "Failed to get data directory, not sure why this should happen, look into dirs::data_local_dir() to find more about error"
                );
                let test_dir = data_dir.join(DEFAULT_FOLDER_NAME).join("tests").join(old_name);
                let new_test_dir = data_dir.join(DEFAULT_FOLDER_NAME).join("tests").join(new_name);
                handle_error!(fs::rename(test_dir, new_test_dir), "Failed to rename test directory");
                self.write_data()
            }
            Some(Commands::CONFIG(args)) => args.run(),
            _ => unreachable!(),
        }
    }

    pub fn load_empty_tests() -> Result<HashMap<String, Test>, String> {
        let data_dir = handle_option!(
            dirs::data_local_dir(),
            "Failed to get data directory, not sure why this should happen, look into dirs::data_local_dir() to find more about error"
        );
        let data_dir = data_dir.join(DEFAULT_FOLDER_NAME);
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).map_err(|e| "Error creating data directory:\n".to_string() + &e.to_string())?;
        }
        // Check for test.json
        // If it exists, load it, if not create it
        let main_path = data_dir.join("test.json");
        let mut tests = HashMap::new();
        if main_path.exists() {
            let metadata = main_path.metadata().map_err(|e| {
                format!(
                    "Error getting metadata for test.json in data dir - {}: \n{}",
                    data_dir.to_str().unwrap(),
                    e.to_string()
                )
            })?;
            if !metadata.is_file() {
                return Err(format!("test.json in {} is not a file", data_dir.to_str().unwrap()));
            }
            let main_file = fs::read_to_string(&main_path).map_err(|e| "Error reading test.json:\n".to_string() + &e.to_string())?;
            let main: HashMap<String, EmptyTest> =
                serde_json::from_str(&main_file).map_err(|e| "Error parsing test.json in data dir:\n".to_string() + &e.to_string())?;
            for (name, empty_test) in main {
                let test = Test::from(empty_test);
                tests.insert(name, test);
            }
        } else {
            let main: HashMap<String, EmptyTest> = HashMap::new();
            let main_file =
                serde_json::to_string_pretty(&main).map_err(|e| "Error serializing test.json in data dir:\n".to_string() + &e.to_string())?;
            fs::write(&main_path, main_file).map_err(|e| "Error writing test.json in data dir:\n".to_string() + &e.to_string())?;
        }
        Ok(tests)
    }

    pub fn clear_temp_files(&self) -> Result<(), String> {
        if let Some(temp_path) = &self.temp_path {
            if temp_path.exists() {
                std::fs::remove_dir_all(temp_path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn write_data(&self) -> Result<(), String> {
        let data_dir = handle_option!(
            dirs::data_local_dir(),
            "Failed to get data directory, not sure why this should happen, look into dirs::data_local_dir() to find more about error"
        );
        let data_dir = data_dir.join(DEFAULT_FOLDER_NAME);
        for (name, test) in self.tests.iter().filter(|(_, test)| !test.is_empty()) {
            let test_path = data_dir.join("tests").join(name);
            if test_path.exists() {
                handle_error!(fs::remove_dir_all(&test_path), "Error removing test directory:")
            } else {
                handle_error!(fs::create_dir_all(&test_path), "Error creating test directory:");
            }
            handle_error!(test.write_data(&test_path), "Error writing test data");
        }
        let main_path = data_dir.join("test.json");
        let main: HashMap<String, EmptyTest> = self.tests.iter().map(|(name, test)| (name.clone(), test.into())).collect();
        let main_file = serde_json::to_string_pretty(&main).map_err(|e| "Error serializing test.json in data dir:\n".to_string() + &e.to_string())?;
        fs::write(&main_path, main_file).map_err(|e| "Error writing test.json in data dir:\n".to_string() + &e.to_string())?;
        Ok(())
    }
}
