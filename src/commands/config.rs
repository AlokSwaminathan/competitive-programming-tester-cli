use clap::{Args, Subcommand};

use crate::{config::Config, handle_error};

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    config_command: ConfigCommands,
}

#[derive(Subcommand, Debug, PartialEq)]
#[allow(non_camel_case_types)]
enum ConfigCommands {
    #[command(about = "Reset the configuration file to default")]
    RESET,

    #[command(about = "Print the configuration file")]
    PRINT,

    #[command(about = "Print the default configuration file")]
    PRINT_DEFAULT,

    #[command(about = "Set the default C++ version")]
    SET_CPP_VER(SetCppVerArgs),

    #[command(about = "Set unicode output")]
    SET_UNICODE(SetUnicodeArgs),

    #[command(about = "Set a flag for g++")]
    SET_GPP_FLAG(SetFlagArgs),

    #[command(about = "Set a flag for gcc")]
    SET_GCC_FLAG(SetFlagArgs),

    #[command(about = "Set a flag for javac")]
    SET_JAVAC_FLAG(SetFlagArgs),

    #[command(about = "Set a flag for java")]
    SET_JAVA_FLAG(SetFlagArgs),

    #[command(about = "Remove a flag for g++")]
    REMOVE_GPP_FLAG(RemoveFlagArgs),

    #[command(about = "Remove a flag for gcc")]
    REMOVE_GCC_FLAG(RemoveFlagArgs),

    #[command(about = "Remove a flag for javac")]
    REMOVE_JAVAC_FLAG(RemoveFlagArgs),

    #[command(about = "Remove a flag for java")]
    REMOVE_JAVA_FLAG(RemoveFlagArgs),

    #[command(about = "Set the default timeout(in milliseconds, 0 for no limit)")]
    SET_TIMEOUT(SetTimeLimitArgs),
}

#[derive(Args, Debug, PartialEq)]
struct SetCppVerArgs {
    #[arg(value_parser=["20","17","14","11"])]
    version: String,
}

#[derive(Args, Debug, PartialEq)]
struct SetUnicodeArgs {
    #[arg(value_parser=is_bool)]
    unicode: i32,
}

fn is_bool(val: &str) -> Result<i32, String> {
    match val.trim().to_ascii_lowercase().as_str() {
        "true" | "t" => Ok(1),
        "false" | "f" => Ok(0),
        _ => Err(format!("\"{}\" is not a valid boolean value", val)),
    }
}

#[derive(Args, Debug, PartialEq)]
struct SetFlagArgs {
    flag: String,
    #[arg(default_value="")]
    value: String,
}

#[derive(Args, Debug, PartialEq)]
struct RemoveFlagArgs {
    flag: String,
}

#[derive(Args, Debug, PartialEq)]
struct SetTimeLimitArgs {
    #[arg(help = "Time in seconds")]
    time: u64,
}

impl ConfigArgs {
    pub fn run(&self) -> Result<(), String> {
        if self.config_command == ConfigCommands::RESET {
            handle_error!(Config::reset(), "Failed to reset config file");
            return Ok(());
        }
        let mut config = handle_error!(Config::get(), "Failed to load config file");
        match &self.config_command {
            ConfigCommands::PRINT => println!("{}", config),
            ConfigCommands::PRINT_DEFAULT => println!("{}", Config::default()),
            ConfigCommands::SET_CPP_VER(args) => {
                let old_val = config.default_cpp_ver;
                config.default_cpp_ver = args.version.parse().unwrap();
                if old_val != config.default_cpp_ver {
                    println!("Overwrote old value: {}", old_val);
                }
            }
            ConfigCommands::SET_UNICODE(args) => {
                let old_val = config.unicode_output;
                config.unicode_output = if args.unicode == 1 { true } else { false };
                if old_val != config.unicode_output {
                    println!("Overwrote old value: {}", old_val)
                };
            }
            ConfigCommands::SET_GPP_FLAG(args) => {
                let old_val = config.gpp_flags.insert(args.flag.clone(), args.value.clone());
                if old_val.is_some() {
                    println!("Overwrote old value: {}", old_val.unwrap());
                }
            }
            ConfigCommands::SET_GCC_FLAG(args) => {
                let old_val = config.gcc_flags.insert(args.flag.clone(), args.value.clone());
                if old_val.is_some() {
                    println!("Overwrote old value: {}", old_val.unwrap());
                }
            }
            ConfigCommands::SET_JAVAC_FLAG(args) => {
                let old_val = config.javac_flags.insert(args.flag.clone(), args.value.clone());
                if old_val.is_some() {
                    println!("Overwrote old value: {}", old_val.unwrap());
                }
            }
            ConfigCommands::SET_JAVA_FLAG(args) => {
                let old_val = config.java_flags.insert(args.flag.clone(), args.value.clone());
                if old_val.is_some() {
                    println!("Overwrote old value: {}", old_val.unwrap());
                }
            }
            ConfigCommands::REMOVE_GPP_FLAG(args) => {
                let old_val = config.gpp_flags.remove(&args.flag);
                if old_val.is_some() {
                    println!("Removed flag");
                } else {
                    println!("Flag not found");
                }
            }
            ConfigCommands::REMOVE_GCC_FLAG(args) => {
                let old_val = config.gcc_flags.remove(&args.flag);
                if old_val.is_some() {
                    println!("Removed flag");
                } else {
                    println!("Flag not found");
                }
            }
            ConfigCommands::REMOVE_JAVAC_FLAG(args) => {
                let old_val = config.javac_flags.remove(&args.flag);
                if old_val.is_some() {
                    println!("Removed flag");
                } else {
                    println!("Flag not found");
                }
            }
            ConfigCommands::REMOVE_JAVA_FLAG(args) => {
                let old_val = config.java_flags.remove(&args.flag);
                if old_val.is_some() {
                    println!("Removed flag");
                } else {
                    println!("Flag not found");
                }
            }
            ConfigCommands::SET_TIMEOUT(args) => {
                let old_val = config.default_timeout;
                config.default_timeout = args.time;
                if old_val != config.default_timeout {
                    println!("Overwrote old value: {}", old_val);
                }
            }
            _ => unreachable!(),
        }
        handle_error!(config.save(), "Failed to save config file");

        Ok(())
    }
}
