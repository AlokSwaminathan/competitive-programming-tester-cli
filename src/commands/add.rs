use crate::program_data::ProgramData;
use crate::test_data::IOType;
use crate::{handle_error, handle_option};
use clap::Args;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use zip::ZipArchive;

const ZIP_BYTES: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const USACO_LINK_PREFIX: &str = "http://www.usaco.org/index.php?page=viewproblem2&cpid=";
const RETURN_TO_PROBLEM_BUTTON_REGEX_STR: &str = r#"<button style=\"margin-bottom:6px;\" type=\"button\" onClick=\"window\.location='index\.php\?page=(?<results>[A-Za-z0-9]+)';\">Return to Problem List</button>"#;
const SOLUTION_BUTTON_REGEX_STR: &str =
    r#"<a href='index\.php\?page=viewproblem2&cpid=(?<id>[0-9]+)'>View problem</a>&nbsp \| &nbsp <a href='(?<test_data>[^']*)'>Test data</a> &nbsp"#;
const PROBLEM_IO_REGEX_STR: &str = r#"INPUT FORMAT \((?<io>[^)]*)\):"#;
const USACO_STANDARD_IO_STR: &str = "input arrives from the terminal / stdin";

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    input_type: InputType,

    #[arg(short, long)]
    #[arg(default_value = "in", requires = "input", help = "Extension of input files, don't use a dot")]
    pub input_extension: String,

    #[arg(short, long)]
    #[arg(default_value = "out", requires = "input", help = "Extension of output files, don't use a dot")]
    pub output_extension: String,

    #[arg(
        short,
        long,
        help = "Defaults to filename of link(foo for https://usaco.org/foo.zip), defaults to folder name for --folder, defaults to zip file name for --usaco-link and --usaco-id"
    )]
    #[arg(requires = "input")]
    pub name: Option<String>,

    #[arg(long, requires = "input", value_delimiter = ',')]
    #[arg(
        help = "Input and output files(Without extension, comma separated), in that order(If you provide only one value, that will be assumed to be the file name for both input and output).\nAssumed to be stdin/stdout unless using usaco link or id, in which case regex will be used to infer it.\nDoesn't support a file input/output and stdin/stdout for the other one, reach out to me if you need this feature"
    )]
    pub io: Option<Vec<String>>,
}

#[derive(Args, Debug, Serialize, Deserialize)]
#[group(required = true, multiple = false)]
struct InputType {
    #[arg(short, long, help = "Supports any link that will download a zip file that extracts to test cases")]
    #[arg(group = "input")]
    link: Option<String>,

    #[arg(short, long, help = "Folder containing test cases")]
    #[arg(group = "input")]
    #[arg(value_parser=validate_folder)]
    folder: Option<PathBuf>,

    #[arg(
        short = 'p',
        long,
        help = "Link to usaco problem page, will download test cases by using regex to get to test data page"
    )]
    #[arg(group = "input")]
    #[arg(value_parser=validate_usaco_link)]
    usaco_link: Option<String>,

    #[arg(
        long,
        help = "ID of usaco problem, is cpid in the link, and will be used to create a link to the problem page"
    )]
    #[arg(group = "input")]
    usaco_id: Option<i32>,
}

fn validate_folder(folder: &str) -> Result<PathBuf, String> {
    let folder = PathBuf::from(folder);
    let exists = folder.try_exists();
    if let Err(e) = exists {
        return Err(e.to_string());
    }

    if !exists.as_ref().unwrap() {
        return Err(String::from("Folder does not exist"));
    }
    if folder.is_file() {
        return Err(String::from("Path is to a file, not a folder"));
    }

    Ok(folder)
}

fn validate_usaco_link(link: &str) -> Result<String, String> {
    if !link.contains(USACO_LINK_PREFIX) {
        return Err(format!(
            "Invalid usaco problem link, must be of the form {}<problem id> (Normal problem links follow this format",
            USACO_LINK_PREFIX
        ));
    }
    Ok(link.to_string())
}

fn get_problem_io(link: &String) -> Result<String, String> {
    let problem_page = handle_error!(reqwest::blocking::get(link), format!("Failed to access problem link: {}", link));
    if problem_page.status() != reqwest::StatusCode::OK {
        return Err(format!(
            "Failed to access link, status code is not 200 it is {}, link: {} ",
            problem_page.status(),
            link
        ));
    }
    let problem_page_text = handle_error!(problem_page.text(), "Failed to get HTML from problem page");
    let io_regex = handle_error!(Regex::new(PROBLEM_IO_REGEX_STR), "Failed to create regex for problem io");
    let io_match = io_regex.captures(&problem_page_text);
    let io_match = handle_option!(
        io_match,
        "Failed to get io from problem page, page doesn't have \"INPUT FORMAT\" section, could mean ID/Link is invalid"
    );
    let io_str = &io_match["io"];
    let io_str = io_str.trim();
    let io_string = if io_str == USACO_STANDARD_IO_STR {
        USACO_STANDARD_IO_STR.to_string()
    } else {
        io_str.split(' ').last().unwrap().to_string()
    };

    Ok(io_string)
}

impl AddArgs {
    pub fn get_test_data(&self) -> Result<(String, PathBuf), String> {
        let result = match (&self.input_type.link,&self.input_type.folder,&self.input_type.usaco_link,&self.input_type.usaco_id) {
            (Some(link),None,None,None) => self.data_from_link(link),
            (None,Some(folder),None,None) => self.data_from_folder(folder),
            (None,None,Some(link),None) => self.data_from_usaco_link(link),
            (None,None,None,Some(id)) => self.data_from_usaco_id(id),
            _ => Err("This means the clap crate has an issue, since it shouldn't allow more than one argument between link, folder, usaco-problem-link, and usaco-problem-id".to_string())
        };
        if self.name.is_none() {
            return result;
        }
        let name = self.name.clone().unwrap();
        let (_, path) = result?;
        Ok((name, path))
    }
    fn data_from_link(&self, link: &String) -> Result<(String, PathBuf), String> {
        let mut response = handle_error!(reqwest::blocking::get(link), "Failed to access link");
        if response.status() != reqwest::StatusCode::OK {
            return handle_error!(
                Err(response.status()),
                format!("Failed to access link, status code is not 200, link: {} ", link)
            );
        }
        let name = link.split("/").last().unwrap().split(".").next().unwrap().to_string().clone();
        let test_names = ProgramData::load_empty_tests().unwrap();
        if test_names.contains_key(&name) {
            return Err(format!("Test with name \"{}\" already exists", &name));
        }

        let mut bytes: Vec<u8> = vec![];
        println!("Downloading zip file...");
        let amount_read = handle_error!(response.copy_to(&mut bytes), "Failed to read response");
        let amount_read_mb = (amount_read as f64) / (1024_f64 * 1024_f64);
        if amount_read_mb < 1.0 {
            println!("Downloaded {:.2} KB successfully", amount_read / 1024);
        } else {
            println!("Downloaded {:.2} MB successfully", amount_read_mb);
        }
        if amount_read < 4 {
            return Err(String::from(
                "Response is not a zip file. First four bytes don't match zip file signature(Less than 4 total bytes in response body).",
            ));
        }
        let is_zip = bytes[0..=3] == ZIP_BYTES;
        if !is_zip {
            return Err(format!(
                "Response is not a zip file. First four bytes in response body don't match zip file signature([{}])",
                &ZIP_BYTES.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<String>>().join(", ")
            ));
        }

        let temp_dir = handle_error!(TempDir::new(), "Failed to create temporary directory to store and extract zip");
        let temp_zip_path = temp_dir.path().join("temp.zip");
        let write_result = fs::write(&temp_zip_path, bytes);
        handle_error!(write_result, "Failed to write zip file to temporary directory");

        let zip_file = handle_error!(fs::File::open(&temp_zip_path), "Failed to open zip file");
        let mut zip_archive = handle_error!(ZipArchive::new(zip_file), "Failed to read zip file");
        handle_error!(zip_archive.extract(temp_dir.path()), "Failed to extract zip file");
        Ok((name, temp_dir.into_path()))
    }
    fn data_from_folder(&self, folder: &PathBuf) -> Result<(String, PathBuf), String> {
        let folder = handle_error!(folder.canonicalize(), "Failed to get canonical(Absolute) path of folder");
        let name = handle_option!(folder.file_name(), "Can't get folder name from folder path, this shouldn't happen").to_str();
        let name = handle_option!(name, "Invalid folder name, not valid utf-8").to_string();
        Ok((name, folder))
    }
    fn data_from_usaco_link(&self, link: &String) -> Result<(String, PathBuf), String> {
        let problem_page = handle_error!(reqwest::blocking::get(link), "Failed to access link");
        if problem_page.status() != reqwest::StatusCode::OK {
            return handle_error!(
                Err(problem_page.status()),
                format!("Failed to access link, status code is not 200, link: {} ", link)
            );
        }
        let problem_page_text = handle_error!(problem_page.text(), "Failed to get HTML from problem page");

        let button_regex = handle_error!(
            Regex::new(RETURN_TO_PROBLEM_BUTTON_REGEX_STR),
            "Failed to create regex for results page button"
        );
        let button_match = button_regex.captures_iter(&problem_page_text).map(|cap| {
            let result = handle_option!(cap.name("results"), "Failed to get results page name from regex capture, page doesn't have \"Return To Problem List\" Button, could mean ID/Link is invalid");
            let result = result.as_str();
            Ok(result)
        }).next();

        let button_match =
            button_match.ok_or("Failed to get results page name from regex capture, page doesn't have \"Return To Problem List\" Button")?;
        let button_match = button_match?;

        let problem_id = link.split("=").last().unwrap().to_string().parse::<i32>();
        let problem_id = handle_error!(problem_id, "Failed to parse problem id from link");
        let results_page_link = format!("http://www.usaco.org/index.php?page={}", button_match);
        let results_page = handle_error!(reqwest::blocking::get(&results_page_link), "Failed to access results page");
        if results_page.status() != reqwest::StatusCode::OK {
            return handle_error!(
                Err(results_page.status()),
                format!("Failed to access link, status code is not 200, link: {} ", link)
            );
        }
        let results_page_text = handle_error!(results_page.text(), "Failed to get HTML from results page");
        let test_data_regex = handle_error!(Regex::new(SOLUTION_BUTTON_REGEX_STR), "Failed to create regex for solution button");
        let test_data_matches: Vec<(i32, String)> = test_data_regex
            .captures_iter(&results_page_text)
            .map(|cap| {
                let id = cap.name("id").expect("Regex error");
                let id = id.as_str().parse::<i32>().unwrap();
                let test_data = cap.name("test_data").expect("Regex error").as_str().to_string();
                (id, test_data)
            })
            .collect();

        let mut test_data_link = None;

        for (id, test_data) in test_data_matches {
            if id == problem_id {
                test_data_link = Some(test_data);
                break;
            }
        }
        if test_data_link.is_none() {
            return Err(format!("Failed to find test data link for problem id {}, at link {}", problem_id, link));
        }
        let test_data_link = test_data_link.unwrap();
        let test_data_link = format!("http://www.usaco.org/{}", test_data_link);
        Ok(self.data_from_link(&test_data_link)?)
    }
    fn data_from_usaco_id(&self, id: &i32) -> Result<(String, PathBuf), String> {
        let link = format!("{}{}", USACO_LINK_PREFIX, id);
        Ok(self.data_from_usaco_link(&link)?)
    }
    pub fn input_type_is_folder(&self) -> bool {
        self.input_type.folder.is_some()
    }
    pub fn get_io(&self) -> Result<(IOType, IOType), String> {
        let mut input_io = IOType::STD;
        let mut output_io = IOType::STD;
        if let Some(io) = &self.io {
            match io.len() {
                1 => {
                    input_io = IOType::FILE(PathBuf::from(&io[0]).with_extension(&self.input_extension));
                    output_io = IOType::FILE(PathBuf::from(&io[0]).with_extension(&self.output_extension));
                }
                2 => {
                    input_io = IOType::FILE(PathBuf::from(&io[0]).with_extension(&self.input_extension));
                    output_io = IOType::FILE(PathBuf::from(&io[1]).with_extension(&self.output_extension));
                }
                _ => return Err("More than 2 values for --io flag, should be 0-2 values".to_string()),
            };
        } else {
            let link: Option<String> = if let Some(usaco_link) = &self.input_type.usaco_link {
                Some(usaco_link.clone())
            } else if let Some(id) = &self.input_type.usaco_id {
                Some(format!("{}{}", USACO_LINK_PREFIX, id))
            } else {
                None
            };

            if link.is_some() {
                let link = link.unwrap();
                let file_name = get_problem_io(&link)?;
                if file_name != USACO_STANDARD_IO_STR {
                    input_io = IOType::FILE(PathBuf::from(&file_name).with_extension(&self.input_extension));
                    output_io = IOType::FILE(PathBuf::from(&file_name).with_extension(&self.output_extension));
                }
            }
        }
        println!("Test IO: {:?}, {:?}", input_io, input_io);

        Ok((input_io, output_io))
    }
}
