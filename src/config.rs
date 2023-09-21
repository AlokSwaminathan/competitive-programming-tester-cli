use std::{collections::HashMap, fmt, fs, process::Command};

use serde::{Deserialize, Serialize};

use crate::{handle_error, handle_option, DEFAULT_FOLDER_NAME};

const DEFAULT_CPP_VER: i32 = 17;
const DEFAULT_TIME_LIMIT: u64 = 5000;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    default_config: Config,
    tags: HashMap<String, Option<Config>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub(crate) default_cpp_ver: i32,
    pub(crate) unicode_output: bool,
    pub(crate) default_timeout: u64,
    pub(crate) gcc_flags: HashMap<String, String>,
    pub(crate) gpp_flags: HashMap<String, String>,
    pub(crate) java_flags: HashMap<String, String>,
    pub(crate) javac_flags: HashMap<String, String>,
}

impl Config {
    pub fn default() -> Config {
        let mut gcc_flags = HashMap::new();
        let mut gpp_flags = HashMap::new();
        let java_flags = HashMap::new();
        let javac_flags = HashMap::new();
        gcc_flags.insert("-O2".to_string(), "".to_string());
        gpp_flags.insert("-O2".to_string(), "".to_string());
        gcc_flags.insert("-lm".to_string(), "".to_string());
        gpp_flags.insert("-lm".to_string(), "".to_string());
        Config {
            gcc_flags,
            gpp_flags,
            java_flags,
            javac_flags,
            default_timeout: DEFAULT_TIME_LIMIT,
            default_cpp_ver: DEFAULT_CPP_VER,
            unicode_output: false,
        }
    }
    pub fn get() -> Result<Config, String> {
        let config_dir = handle_option!(
            dirs::config_local_dir(),
            "Failed to get config directory, not sure why this should happen, look into dirs::config_local_dir() to find more about error"
        );
        let config_dir = config_dir.join(DEFAULT_FOLDER_NAME);
        if !config_dir.exists() {
            handle_error!(fs::create_dir_all(&config_dir), "Failed to create config directory");
        }
        if !config_dir.is_dir() {
            return Err(format!("Config directory: {:?} is not a directory", config_dir));
        }
        let config_path = config_dir.join("config.json");
        let config: Config = if config_path.exists() {
            let config_file = handle_error!(fs::read_to_string(&config_path), "Failed to read config file");
            handle_error!(serde_json::from_str(&config_file), "Failed to parse config file")
        } else {
            let config = Config::default();
            let config_file = handle_error!(serde_json::to_string_pretty(&config), "Failed to serialize config file");
            handle_error!(fs::write(&config_path, config_file), "Failed to write config file");
            config
        };

        Ok(config)
    }
    pub fn get_cpp_ver() -> &'static str {
        let config = Config::get();
        let cpp_ver = if let Ok(conf) = config {
            let cpp_ver = conf.default_cpp_ver.clone();
            cpp_ver.to_string()
        } else {
            DEFAULT_CPP_VER.to_string()
        };
        Box::leak(cpp_ver.into_boxed_str())
    }
    pub fn get_time_limit() -> &'static str {
        let config = Config::get();
        let time_limit = if let Ok(conf) = config {
            let time_limit = conf.default_timeout.clone();
            time_limit.to_string()
        } else {
            DEFAULT_TIME_LIMIT.to_string()
        };
        Box::leak(time_limit.into_boxed_str())
    }
    pub fn get_gcc_command(&self) -> Command {
        let mut command = Command::new("gcc");
        for (flag, value) in self.gcc_flags.iter() {
            command.arg(format!("{}{}{}", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        command
    }
    pub fn get_gpp_command(&self) -> Command {
        let mut command = Command::new("g++");
        for (flag, value) in self.gpp_flags.iter() {
            command.arg(format!("{}{}{}", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        command
    }
    pub fn get_java_command(&self) -> Command {
        let mut command = Command::new("java");
        for (flag, value) in self.java_flags.iter() {
            command.arg(format!("{}{}{}", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        command
    }
    pub fn get_javac_command(&self) -> Command {
        let mut command = Command::new("javac");
        for (flag, value) in self.javac_flags.iter() {
            command.arg(format!("{}{}{}", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        command
    }
    pub fn reset() -> Result<(), String> {
        let config_dir = handle_option!(
            dirs::config_local_dir(),
            "Failed to get config directory, not sure why this should happen, look into dirs::config_local_dir() to find more about error"
        );
        let config_dir = config_dir.join(DEFAULT_FOLDER_NAME);
        if !config_dir.exists() {
            handle_error!(fs::create_dir_all(&config_dir), "Failed to create config directory");
        }
        if !config_dir.is_dir() {
            return Err(format!("Config directory: {:?} is not a directory", config_dir));
        }
        let config_path = config_dir.join("config.json");
        let config = Config::default();
        let config_file = handle_error!(serde_json::to_string_pretty(&config), "Failed to serialize config file");
        handle_error!(fs::write(&config_path, config_file), "Failed to write config file");
        println!("Config file reset to default");
        Ok(())
    }
    pub fn get_unicode_output(&self) -> bool {
        self.unicode_output
    }
    pub fn save(&self) -> Result<(), String> {
        let config_dir = handle_option!(
            dirs::config_local_dir(),
            "Failed to get config directory, not sure why this should happen, look into dirs::config_local_dir() to find more about error"
        );
        let config_dir = config_dir.join(DEFAULT_FOLDER_NAME);
        if !config_dir.exists() {
            handle_error!(fs::create_dir_all(&config_dir), "Failed to create config directory");
        }
        if !config_dir.is_dir() {
            return Err(format!("Config directory: {:?} is not a directory", config_dir));
        }
        let config_path = config_dir.join("config.json");
        let config_file = handle_error!(serde_json::to_string_pretty(&self), "Failed to serialize config file");
        handle_error!(fs::write(&config_path, config_file), "Failed to write config file");
        Ok(())
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut gcc_flags = vec![];
        let mut gpp_flags = vec![];
        let mut java_flags = vec![];
        let mut javac_flags = vec![];
        for (flag, value) in self.gcc_flags.iter() {
            gcc_flags.push(format!("\"{}{}{}\"", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        for (flag, value) in self.gpp_flags.iter() {
            gpp_flags.push(format!("\"{}{}{}\"", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        for (flag, value) in self.java_flags.iter() {
            java_flags.push(format!("\"{}{}{}\"", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        for (flag, value) in self.javac_flags.iter() {
            javac_flags.push(format!("\"{}{}{}\"", flag, if value.is_empty() { "" } else { "=" }, value));
        }
        gcc_flags.sort_unstable();
        gpp_flags.sort_unstable();
        java_flags.sort_unstable();
        javac_flags.sort_unstable();

        let gcc_flags = gcc_flags.join(", ");
        let gpp_flags = gpp_flags.join(", ");
        let java_flags = java_flags.join(", ");
        let javac_flags = javac_flags.join(", ");

        write!(
            f,
            "Default C++ version: {}\nUnicode output: {}\nDefault time limit: {} ms\nGCC flags: {}\nG++ flags: {}\nJava flags: {}\nJavac flags: {}\n",
            self.default_cpp_ver, self.unicode_output, self.default_timeout, gcc_flags, gpp_flags, java_flags, javac_flags
        )
    }
}
