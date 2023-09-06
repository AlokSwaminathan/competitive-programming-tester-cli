use crate::{config::Config, handle_error, handle_option, test_data::Test};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

use clap::Args;
use tempfile::TempDir;
use wait_timeout::ChildExt;

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(help = "The name of the test to run")]
    pub test: String,

    #[arg(
        short,
        long,
        value_delimiter = ',',
        help = "The name of the test case to run. \nIf multiple test cases are specified(Use a comma between cases), all of them will be run. \nIf not specified, all test cases will be run"
    )]
    pub cases: Option<Vec<String>>,

    #[arg(short, long, help = "Show input for each test case(Input can be very large)")]
    pub show_input: bool,

    #[arg(short = 'o', long, help = "Compare output of program to desired output")]
    pub compare_output: bool,

    #[arg(short,long,value_parser=file_exists,help="The file to run, should be a file with a valid extension(.c, .cpp, .java, .py)")]
    pub file: PathBuf,

    #[arg(long,default_value=Config::get_cpp_ver(),value_parser=["20","17","14","11"],help="The C++ version to compile with, default is the version in the config file, else 17")]
    pub cpp_ver: String,

    #[arg(short,long,default_value=Config::get_time_limit(),help="The time limit for each test case, in milliseconds, default is the time limit in the config file, else 1000")]
    pub timeout: u64,
}

pub enum FileType {
    C,
    CPP(i32),
    JAVA,
    PYTHON,
}

#[derive(Debug)]
struct RunCommand(Command);

#[derive(Debug)]
pub struct RunDir {
    temp_dir: TempDir,
    run_command: RunCommand,
    input_file: Option<PathBuf>,
    output_file: Option<PathBuf>,
    show_input: bool,
    compare_output: bool,
    test: Test,
    unicode_output: bool,
    timeout: u64,
}

fn file_exists(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    if !path.exists() {
        return Err(format!("There is no file at path: \"{}\"", file));
    } else {
        let metadata = handle_error!(fs::metadata(path.clone()), "Failed to get metadata for path, despite its existence");
        if !metadata.is_file() {
            return Err(format!(
                "Path: \"{}\", is a path to a folder, --file takes a file(As the name implies)",
                file
            ));
        } else {
            let extension = handle_option!(path.extension(), "Failed to get file extension");
            let extension = handle_option!(extension.to_str(), "Failed to turn file extension into valid UTF-8");

            match extension {
                "cpp" | "java" | "py" | "c" => (),
                _ => {
                    return Err(format!(
                        "File has extension \".{}\", which is invalid, program only supports C(.c), C++(.cpp), Java(.java), and Python(.py)",
                        extension
                    ))
                }
            };
        }
    }
    let path = handle_error!(path.canonicalize(), "Failed to canonicalize(Find absolute path), to file");
    Ok(path)
}

impl RunDir {
    pub fn new(test: &Test, args: &RunArgs, config: &Config) -> Result<RunDir, String> {
        let mut test = test.clone();
        test.set_cases(&args.cases)?;
        let temp_dir = handle_error!(TempDir::new(), "Failed to create temporary directory");
        let temp_dir_path = temp_dir.path().to_path_buf();
        let run_command = RunCommand::new(&temp_dir_path, &args.file, &args.cpp_ver, &config)?;
        let (input_file, output_file) = test.get_files(&temp_dir_path);
        Ok(RunDir {
            temp_dir,
            run_command,
            input_file,
            output_file,
            show_input: args.show_input,
            compare_output: args.compare_output,
            test: test,
            unicode_output: config.get_unicode_output(),
            timeout: args.timeout,
        })
    }
    pub fn run(&mut self) -> Result<(), String> {
        for (name, case) in self.test.case_iter() {
            let run_command = &mut self.run_command.0;
            if let Some(file) = &self.input_file {
                case.write_input(file, name)?;
            } else {
                let input_path = self.temp_dir.path().join("tmp.in");
                case.write_input(&input_path, name)?;
                let input_file = handle_error!(File::open(input_path), "Failed to open input file");
                run_command.stdin(input_file);
            }
            run_command.current_dir(self.temp_dir.path());
            let timeout = Duration::from_millis(self.timeout);

            let mut run_command = handle_error!(run_command.spawn(), "Failed to spawn thread for program");
            let now = Instant::now();
            let output = handle_error!(run_command.wait_timeout(timeout), "Failed to wait for program to finish");
            let exit_status = match output {
                Some(output) => output,
                None => {
                    return Err(format!(
                        "\nProgram timed out after {} milliseconds, if you want to change the timeout, use the --timeout flag",
                        self.timeout
                    ))
                }
            };
            let time_taken = now.elapsed().as_millis();

            // let output = handle_error!(run_command.output(), "Failed to run program");

            if !exit_status.success() {
                return Err(format!("\nProgram exited with non-zero exit code: {}", exit_status.code().unwrap()));
            }
            let output = if let Some(file) = &self.output_file {
                handle_error!(fs::read(file), "\nFailed to read from output file, test case")
            } else {
                run_command.stdout.take().unwrap().bytes().map(|b| b.unwrap()).collect::<Vec<u8>>()
            };
            let output = handle_error!(String::from_utf8(output), "Failed to turn output into valid UTF-8");
            print!("Test Case {}: ", name);
            handle_error!(io::stdout().flush(), "\nFailed to flush stdout");
            if self.show_input {
                println!();
                println!("Input:");
                println!(
                    "{}",
                    case.get_input().lines().map(|l| format!("\t{}", l)).collect::<Vec<String>>().join("\n")
                );
            }
            if self.compare_output {
                println!();
                println!("Correct Output:");
                println!(
                    "{}",
                    case.get_output().lines().map(|l| format!("\t{}", l)).collect::<Vec<String>>().join("\n")
                );
                println!("Program Output:");
                println!("{}", output.lines().map(|l| format!("\t{}", l)).collect::<Vec<String>>().join("\n"));
            }
            println!("Time Taken: {} milliseconds", time_taken);
            let pass_symbol = match self.unicode_output {
                true => "✅",
                false => "PASSED",
            };
            let fail_symbol = match self.unicode_output {
                true => "\x1b[31m❌\x1b[0m",
                false => "FAILED",
            };
            if case.get_output().trim() == output.trim() {
                println!("{pass_symbol}");
            } else {
                println!("{fail_symbol}");
            }
        }
        Ok(())
    }
}

impl RunCommand {
    fn new(temp_path: &PathBuf, file_path: &PathBuf, cpp_ver: &String, config: &Config) -> Result<Self, String> {
        let file_type = match file_path.extension().unwrap().to_str().unwrap() {
            "cpp" => FileType::CPP(cpp_ver.parse().unwrap()),
            "c" => FileType::C,
            "java" => FileType::JAVA,
            "py" => FileType::PYTHON,
            _ => unreachable!("Invalid file extension"),
        };
        let run_command = match file_type {
            FileType::CPP(ver) => {
                let mut compile_command = config.get_gpp_command();
                compile_command.arg("-o").arg(temp_path.join("output"));
                compile_command.arg(format!("-std=c++{}", ver));
                compile_command.arg(file_path);
                let output = handle_error!(compile_command.output(), "Failed to compile file");
                if !output.status.success() {
                    return Err(format!(
                        "Failed to compile file, exited with non-zero exit code: {}\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap(),
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                // Using local address then will use env to make the location the temp dir, so it looks for files in the temp dir
                let run_command = Command::new("./output");
                run_command
            }
            FileType::C => {
                let mut compile_command = config.get_gcc_command();
                compile_command.arg("-o").arg(temp_path.join("output"));
                compile_command.arg(file_path);
                handle_error!(compile_command.output(), "Failed to compile file");
                let run_command = Command::new("./output");
                run_command
            }
            FileType::JAVA => {
                let mut compile_command = Command::new("javac");
                compile_command.arg(file_path);
                compile_command.arg("-d").arg(temp_path);
                handle_error!(compile_command.output(), "Failed to compile file");
                let mut class_name = temp_path.join(file_path.file_stem().unwrap());
                class_name.set_extension("class");
                if !class_name.exists() {
                    return Err(format!(
                        "Failed to find class file: \"{}\". Class name should be the same as the file name",
                        class_name.to_str().unwrap()
                    ));
                }
                let mut run_command = Command::new("java");
                run_command.arg(class_name.file_name().unwrap());
                run_command
            }
            FileType::PYTHON => {
                let mut run_command = Command::new("python3");
                run_command.arg("-O");
                run_command.arg(file_path);
                run_command
            }
        };
        Ok(RunCommand(run_command))
    }
}
