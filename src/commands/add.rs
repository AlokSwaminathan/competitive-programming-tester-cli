use crate::program_data::ProgramData;
use crate::test_data::IOType;
use crate::{handle_error, handle_option};
use clap::Args;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use zip::ZipArchive;

const ZIP_BYTES: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const USACO_LINK_PREFIX: &str = "http://www.usaco.org/index.php?page=viewproblem2&cpid=";
const CODEFORCES_LINK_PREFIX: &str = "https://codeforces.com/problemset/problem/";
const CODEFORCES_LINK_ALTERNATE_PREFIX: &str = "https://codeforces.com/contest/";
const ATCODER_LINK_PREFIX: &str = "https://atcoder.jp/contests/";
const USACO_RETURN_TO_PROBLEM_BUTTON_REGEX_STR: &str = r#"<button style=\"margin-bottom:6px;\" type=\"button\" onClick=\"window\.location='index\.php\?page=(?<results>[A-Za-z0-9]+)';\">Return to Problem List</button>"#;
const USACO_TEST_DATA_BUTTON_REGEX_STR: &str =
    r#"<a href='index\.php\?page=viewproblem2&cpid=(?<id>[0-9]+)'>View problem</a>&nbsp \| &nbsp <a href='(?<test_data>[^']*)'>Test data</a> &nbsp"#;
const PROBLEM_IO_REGEX_STR: &str = r#"INPUT FORMAT \((?<io>[^)]*)\):"#;
const USACO_STANDARD_IO_STR: &str = "input arrives from the terminal / stdin";
const USACO_PROBLEM_NAME_REGEX_STR: &str = r#"<h2> (?<description>USACO 20(?<year>\d\d) (?<competition>.+), (?<divison>.+) <\/h2>
<h2> Problem \d\. (?<name>.+)) <\/h2>"#;
const USACO_EXAMPLE_PROBLEM_STR: &str = r#"<h4>SAMPLE INPUT:<\/h4>.*?<pre class='in'>\n(?<input>(.|\n)*?)<\/pre>.*?<h4>SAMPLE OUTPUT:<\/h4>.*?<pre class='out'>\n(?<output>(.|\n)*?)<\/pre>"#;
const ATCODER_NAME_REGEX_STR: &str = r#"<span class="h2">(?<name>((.|\n)*?))<"#;
const ATCODER_DESCRIPTION_REGEX_STR: &str = r#"<a class="contest-title".*?>(?<contest_info>(.*?))<\/a>"#;
const CODEFORCES_NAME_REGEX_STR: &str = r#"<div class="title">(?<name>((.|\n)*?))<"#;
const CODEFORCES_DESCRIPTION_REGEX_STR: &str = r#"<a style="color: black" href=".*?">(?<contest_info>(.|\n)*?)<\/a>"#;

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
        help = "Defaults to filename of link(foo for https://usaco.org/foo.zip), defaults to folder name for --folder, inferred for USACO, Codeforces, and AtCoder links"
    )]
    #[arg(requires = "input")]
    pub name: Option<String>,

    #[arg(long, requires = "input", value_delimiter = ',')]
    #[arg(
        help = "FYI: this is unnecessary if links are from USACO, Codeforces, or AtCoder.\nInput and output files(Without extension, comma separated), in that order(If you provide only one value, that will be assumed to be the file name for both input and output).\nAssumed to be stdin/stdout unless using usaco link or id, in which case regex will be used to infer it(Is stdin/stdout for Codeforces and AtCoder).\nDoesn't support a file input/output and stdin/stdout for the other one, reach out to me if you need this feature"
    )]
    pub io: Option<Vec<String>>,

    #[arg(long, short, requires = "input")]
    #[arg(
        help = "Optional. Description of test, will be shown when listing tests (Overrides inference). Inferred for USACO, Codeforces, and AtCoder links"
    )]
    pub description: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SubmissionType {
    USACO,
    CODEFORCES,
    ATCODER,
}

impl Display for SubmissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            SubmissionType::USACO => "USACO",
            SubmissionType::CODEFORCES => "Codeforces",
            SubmissionType::ATCODER => "AtCoder",
        };
        write!(f, "{}", string)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubmissionData {
    pub submission_type: SubmissionType,
    pub link: String,
}

impl SubmissionData {
    pub fn try_from_link(link: &String) -> Option<SubmissionData> {
        let submission_type = if link.contains(USACO_LINK_PREFIX) {
            Some(SubmissionType::USACO)
        } else if link.contains(CODEFORCES_LINK_PREFIX) || link.contains(CODEFORCES_LINK_ALTERNATE_PREFIX) {
            Some(SubmissionType::CODEFORCES)
        } else if link.contains(ATCODER_LINK_PREFIX) {
            Some(SubmissionType::ATCODER)
        } else {
            None
        };
        if submission_type.is_none() {
            return None;
        }
        let submission_type = submission_type.unwrap();
        Some(SubmissionData {
            submission_type,
            link: link.clone(),
        })
    }

    pub fn get_data_link(&self) -> Result<String, String> {
        match self.submission_type {
            SubmissionType::USACO => self.usaco_data_link(),
            _ => unreachable!(),
        }
    }

    pub fn get_test_name(&self) -> Result<String, String> {
        match self.submission_type {
            SubmissionType::USACO => self.usaco_test_name(),
            SubmissionType::CODEFORCES => self.codeforces_test_name(),
            SubmissionType::ATCODER => self.atcoder_test_name(),
        }
    }

    pub fn get_test_description(&self) -> Result<String, String> {
        match self.submission_type {
            SubmissionType::USACO => self.usaco_test_description(),
            SubmissionType::CODEFORCES => self.codeforces_test_description(),
            SubmissionType::ATCODER => self.atcoder_test_description(),
        }
    }

    pub fn get_data(&self) -> Result<PathBuf, String> {
        match self.submission_type {
            SubmissionType::ATCODER => self.atcoder_data(),
            SubmissionType::CODEFORCES => self.codeforces_data(),
            SubmissionType::USACO => unreachable!(),
        }
    }

    pub fn get_io(&self, input_extension: &String, output_extension: &String) -> Result<(IOType, IOType), String> {
        match self.submission_type {
            SubmissionType::USACO => self.usaco_io(input_extension, output_extension),
            SubmissionType::CODEFORCES => Ok((IOType::STD, IOType::STD)),
            SubmissionType::ATCODER => Ok((IOType::STD, IOType::STD)),
        }
    }

    fn usaco_io(&self, input_extension: &String, output_extension: &String) -> Result<(IOType, IOType), String> {
        let problem_page = handle_error!(
            reqwest::blocking::get(&self.link),
            format!("Failed to access problem link: {}", self.link)
        );
        if problem_page.status() != reqwest::StatusCode::OK {
            return Err(format!(
                "Failed to access link, status code is not 200 it is {}, link: {} ",
                problem_page.status(),
                self.link
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

        let input_io = if io_string == USACO_STANDARD_IO_STR {
            IOType::STD
        } else {
            IOType::FILE(PathBuf::from(&io_string).with_extension(&input_extension))
        };
        let output_io = if io_string == USACO_STANDARD_IO_STR {
            IOType::STD
        } else {
            IOType::FILE(PathBuf::from(&io_string).with_extension(&output_extension))
        };
        Ok((input_io, output_io))
    }

    pub fn write_usaco_examples(&self, write_path: PathBuf, input_extension: &String, output_extension: &String) -> Result<(), String> {
        let problem_page = handle_error!(
            reqwest::blocking::get(&self.link),
            format!("Failed to access problem link: {}", self.link)
        );
        if problem_page.status() != reqwest::StatusCode::OK {
            return Err(format!(
                "Failed to access link, status code is not 200 it is {}, link: {} ",
                problem_page.status(),
                self.link
            ));
        }
        let problem_page_text = handle_error!(problem_page.text(), "Failed to get HTML from problem page");

        let example_regex = handle_error!(Regex::new(USACO_EXAMPLE_PROBLEM_STR), "Failed to create regex for example problem");

        let example_matches: Vec<(String, String)> = example_regex
            .captures_iter(&problem_page_text)
            .map(|cap| {
                let input = cap.name("input").expect("Regex error").as_str().to_string();
                let output = cap.name("output").expect("Regex error").as_str().to_string();
                (input, output)
            })
            .collect();

        for (i, (input, output)) in example_matches.iter().enumerate() {
            let input_path = write_path.join(format!("example{}.{}", i + 1, input_extension));
            let output_path = write_path.join(format!("example{}.{}", i + 1, output_extension));
            handle_error!(fs::write(&input_path, input), "Failed to write example input");
            handle_error!(fs::write(&output_path, output), "Failed to write example output");
        }

        Ok(())
    }

    fn atcoder_test_name(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;
        let name_regex = handle_error!(
            Regex::new(ATCODER_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", ATCODER_NAME_REGEX_STR)
        );
        let contest_name_task = {
            let cutoff_link = &self.link.split("/tasks/").last();
            let cutoff_link = handle_option!(
                cutoff_link,
                "Failed to get get contest name from link, leave github issue, probably mean atcoder link format has changed"
            );
            cutoff_link
        };
        let (_name,formatted_name) = handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let name = handle_option!(cap.name("name"), "Failed to get name of problem from problem page using regex");
                    let formatted_name = name.as_str().trim().split("-").last().unwrap().split("(").next().unwrap().trim().replace(" ","_").replace("\n","_").to_ascii_lowercase();
                    Ok((name.as_str(),formatted_name))
                })
                .next(),
            "Failed to infer name from AtCoder problem page, please leave a github issue and pass a name when adding the test to make it work for now"
        )?;
        Ok(format!("{}_{}", formatted_name, contest_name_task))
    }

    fn atcoder_test_description(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;
        let name_regex = handle_error!(
            Regex::new(ATCODER_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", ATCODER_NAME_REGEX_STR)
        );
        let description_regex = handle_error!(
            Regex::new(ATCODER_DESCRIPTION_REGEX_STR),
            format!("Failed to create regex from string - String is {}", ATCODER_DESCRIPTION_REGEX_STR)
        );
        let unformatted_name = handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let name = handle_option!(cap.name("name"), "Failed to get name of problem from problem page using regex");
                    Ok(name.as_str())
                })
                .next(),
            "Failed to infer name from AtCoder problem page, please leave a github issue and pass a name when adding the test to make it work for now"
        )?;
        let description = handle_option!(
            description_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let description = handle_option!(cap.name("contest_info"), "Failed to get description of problem from problem page using regex");
                    Ok(description.as_str().trim().to_string())
                })
                .next(),
            "Failed to infer description from AtCoder problem page, please leave a github issue and pass a description when adding the test to make it work for now"
        )?;
        let description = format!("{}: {}", description.trim(), unformatted_name.trim());
        Ok(description)
    }

    fn atcoder_data(&self) -> Result<PathBuf, String> {
        unimplemented!()
    }

    fn codeforces_test_name(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;
        let name_regex = handle_error!(
            Regex::new(CODEFORCES_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", CODEFORCES_NAME_REGEX_STR)
        );
        let name = handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let name = handle_option!(cap.name("name"), "Failed to get name of problem from problem page using regex");
                    Ok(name.as_str().trim().replace(" ", "_").replace("/n","_").replace(".","").to_ascii_lowercase())
                })
                .next(),
            "Failed to infer name from Codeforces problem page, please leave a github issue and pass a name when adding the test to make it work for now"
        )?;
        Ok(name)
    }

    fn codeforces_test_description(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;
        let description_regex = handle_error!(
            Regex::new(CODEFORCES_DESCRIPTION_REGEX_STR),
            format!("Failed to create regex from string - String is {}", CODEFORCES_DESCRIPTION_REGEX_STR)
        );
        let description = handle_option!(
            description_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let description = handle_option!(cap.name("contest_info"), "Failed to get description of problem from problem page using regex");
                    Ok(description.as_str().trim().to_string())
                })
                .next(),
            "Failed to infer description from Codeforces problem page, please leave a github issue"
        )?;
        let name_regex = handle_error!(
            Regex::new(CODEFORCES_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", CODEFORCES_NAME_REGEX_STR)
        );
        let name = handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let name = handle_option!(cap.name("name"), "Failed to get name of problem from problem page using regex");
                    Ok(name.as_str().trim().to_string())
                })
                .next(),
            "Failed to infer name for description from Codeforces problem page, please leave a github issue"
        )?;
        Ok(format!("{}: {} (Examples only)", description, name))
    }

    fn codeforces_data(&self) -> Result<PathBuf, String> {
        unimplemented!()
    }

    fn usaco_test_name(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;

        let name_regex = handle_error!(
            Regex::new(USACO_PROBLEM_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", USACO_PROBLEM_NAME_REGEX_STR)
        );
        let name =
            handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let year = handle_option!(cap.name("year"), "Failed to get year of contest from problem page using regex");
                    let competition = handle_option!(cap.name("competition"), "Failed to get name of contest from problem page using regex");
                    let divison = handle_option!(cap.name("divison"), "Failed to get divison of contest from problem page using regex");
                    let name = handle_option!(cap.name("name"), "Failed to get name of problem from problem page using regex");
                    let competition = competition.as_str().trim().to_ascii_lowercase();
                    let competition = if competition.contains("us open") { "open" } else { &competition[0..3] };
                    Ok(format!(
                        "{}_{}_{}{}",
                        if name.as_str().contains("Contest") {
                            name.as_str().split("Contest").next().unwrap().trim().replace(" ", "_").to_ascii_lowercase()
                        } else {
                            name.as_str().trim().replace(" ", "_").to_ascii_lowercase()
                        },
                        divison.as_str().trim().to_ascii_lowercase(),
                        competition,
                        year.as_str().trim()
                    ))
                })
                .next(),
            "Failed to infer name from USACO problem page, please leave a github issue and pass a name when adding the test to make it work for now"
        )?;
        Ok(name)
    }

    fn usaco_test_description(&self) -> Result<String, String> {
        let problem_page_text = get_link_html(&self.link)?;

        let name_regex = handle_error!(
            Regex::new(USACO_PROBLEM_NAME_REGEX_STR),
            format!("Failed to create regex from string - String is {}", USACO_PROBLEM_NAME_REGEX_STR)
        );
        let description =
            handle_option!(
            name_regex
                .captures_iter(&problem_page_text)
                .map(|cap| {
                    let description = handle_option!(cap.name("description"), "Failed to get description of problem from problem page using regex");
                    let description = description.as_str().trim().replace(" </h2>\n<h2>", ":");
                    Ok(description)
                })
                .next(),
            "Failed to infer name from USACO problem page, please leave a github issue and pass a name when adding the test to make it work for now"
        )?;
        Ok(description)
    }

    fn usaco_data_link(&self) -> Result<String, String> {
        let link = &self.link;
        let problem_page = handle_error!(reqwest::blocking::get(link), "Failed to access link");
        if problem_page.status() != reqwest::StatusCode::OK {
            return handle_error!(
                Err(problem_page.status()),
                format!("Failed to access link, status code is not 200, link: {} ", link)
            );
        }
        let problem_page_text = handle_error!(problem_page.text(), "Failed to get HTML from problem page");

        let button_regex = handle_error!(
            Regex::new(USACO_RETURN_TO_PROBLEM_BUTTON_REGEX_STR),
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
        let test_data_regex = handle_error!(Regex::new(USACO_TEST_DATA_BUTTON_REGEX_STR), "Failed to create regex for solution button");
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
        Ok(test_data_link)
    }
}

impl AddArgs {
    pub fn get_test_data(&self) -> Result<(String, PathBuf, Option<SubmissionData>, Option<String>), String> {
        match (&self.input_type.link, &self.input_type.folder, &self.input_type.usaco_id) {
            (Some(link), None, None) => self.data_from_link(link),
            (None, Some(folder), None) => self.data_from_folder(folder),
            (None, None, Some(id)) => self.data_from_usaco_id(id),
            _ => Err(
                "This means the clap crate has an issue, since it shouldn't allow more than one argument between link, folder, and usaco-problem-id"
                    .to_string(),
            ),
        }
    }
    fn data_from_link(&self, link: &String) -> Result<(String, PathBuf, Option<SubmissionData>, Option<String>), String> {
        let submission_data = SubmissionData::try_from_link(link);
        let submission_name = if self.name.is_some() {
            None
        } else if let Some(submission_data) = submission_data.as_ref() {
            Some(submission_data.get_test_name()?)
        } else {
            None
        };
        let submission_description = if self.description.is_some() {
            None
        } else if let Some(submission_data) = submission_data.as_ref() {
            let desc = submission_data.get_test_description();
            if let Err(err) = &desc {
                println!("Failed to get description from link, using no description for now, leave a github issue for fix. Error info: \n {}",err);
            }
            desc.ok()
        } else {
            None
        };
        let name = &self
            .name
            .as_ref()
            .or(submission_name.as_ref())
            .clone()
            .unwrap_or(&link.split("/").last().unwrap().split(".").next().unwrap().to_string().clone())
            .clone();
        let description = &self.description.as_ref().or(submission_description.as_ref()).cloned();
        let name = name.clone();
        let description = description.clone();
        println!("Test Name: {}\nTest Description: {}", name, description.as_ref().unwrap_or(&"Nothing".to_string()));

        if submission_data.is_some() && submission_data.as_ref().unwrap().submission_type != SubmissionType::USACO {
            let data_path = handle_error!(
                submission_data.as_ref().unwrap().get_data(),
                format!(
                    "Failed to get data from link for submission type: {}",
                    submission_data.unwrap().submission_type
                )
            );
            return Ok((name, data_path, submission_data, description));
        }

        let link = &if submission_data.is_some() {
            handle_error!(submission_data.as_ref().unwrap().get_data_link(), "Failed to get link for test data")
        } else {
            link.clone()
        };

        let mut response = handle_error!(reqwest::blocking::get(link), "Failed to access link");
        if response.status() != reqwest::StatusCode::OK {
            return handle_error!(
                Err(response.status()),
                format!("Failed to access link, status code is not 200, link: {} ", link)
            );
        }

        println!("Test name is \"{}\"", name);
        if submission_data.is_some() {
            println!("Submission type is {}", submission_data.as_ref().unwrap().submission_type);
        } else {
            println!("No submission type(USACO, Codeforces, and AtCoder are supported and should be inferred if given links to the problem page)");
        }
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
        if let Some(submission_data) = submission_data.as_ref() {
            if submission_data.submission_type == SubmissionType::USACO {
                submission_data.write_usaco_examples(temp_dir.path().to_path_buf(), &self.input_extension, &self.output_extension)?;
            }
        }
        Ok((name, temp_dir.into_path(), submission_data, description))
    }
    fn data_from_folder(&self, folder: &PathBuf) -> Result<(String, PathBuf, Option<SubmissionData>, Option<String>), String> {
        let folder = handle_error!(folder.canonicalize(), "Failed to get canonical(Absolute) path of folder");
        let name = if self.name.is_some() {
            self.name.as_ref().unwrap().clone()
        } else {
            let name = handle_option!(folder.file_name(), "Can't get folder name from folder path, this shouldn't happen").to_str();
            let name = handle_option!(name, "Invalid folder name, not valid utf-8").to_string();
            name
        };
        let test_names = ProgramData::load_empty_tests().unwrap();
        if test_names.contains_key(&name) {
            return Err(format!("Test with name \"{}\" already exists", &name));
        }
        println!("Test name is \"{}\"", name);
        let description = if self.description.is_some() { self.description.clone() } else { None };
        Ok((name, folder, None, description))
    }

    fn data_from_usaco_id(&self, id: &i32) -> Result<(String, PathBuf, Option<SubmissionData>, Option<String>), String> {
        let link = format!("{}{}", USACO_LINK_PREFIX, id);
        self.data_from_link(&link)
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
            let submission_data = if let Some(id) = &self.input_type.usaco_id {
                let link = format!("{}{}", USACO_LINK_PREFIX, id);
                SubmissionData::try_from_link(&link)
            } else if let Some(link) = &self.input_type.link {
                SubmissionData::try_from_link(link)
            } else {
                None
            };
            if let Some(submission_data) = submission_data {
                (input_io, output_io) = submission_data.get_io(&self.input_extension, &self.output_extension)?;
            }

            // if link.is_some() {
            //     let link = link.unwrap();
            //     let file_name = get_problem_io(&link)?;
            //     if file_name != USACO_STANDARD_IO_STR {
            //         input_io = IOType::FILE(PathBuf::from(&file_name).with_extension(&self.input_extension));
            //         output_io = IOType::FILE(PathBuf::from(&file_name).with_extension(&self.output_extension));
            //     }
            // }
        }
        println!("Test IO: {:?}, {:?}", input_io, input_io);

        Ok((input_io, output_io))
    }
}

fn get_link_html(link: &String) -> Result<String, String> {
    let problem_page = handle_error!(reqwest::blocking::get(link), format!("Failed to access problem link: {}", link));
    if problem_page.status() != reqwest::StatusCode::OK {
        return Err(format!(
            "Failed to access link, status code is not 200 it is {}, link: {} ",
            problem_page.status(),
            link
        ));
    }
    let problem_page_text = handle_error!(problem_page.text(), "Failed to get HTML from problem page");
    Ok(problem_page_text)
}
