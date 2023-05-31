use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use configparser::ini::Ini;

const CONFIG_FOLDER: &'static str = "indiserver_ui";
const CONFIG_FILENAME: &'static str = "config.ini";

pub fn fetch_indi_binaries() -> Vec<(String, String, bool)> {
    let ls = Command::new("ls")
        .arg("/usr/bin/")
        .stdout(Stdio::piped())
        .spawn()
        .expect("ls command failed");

    let grep = Command::new("grep")
        .arg("^indi")
        .stdin(ls.stdout.unwrap())
        .output()
        .expect("Fail to grep");

    let result = String::from_utf8(grep.stdout).unwrap();
    let split: Vec<&str> = result.split("\n").collect();

    let mut content = Vec::with_capacity(split.len() - 1);

    for name in split.into_iter() {
        if name != "indiserver" && name != "" {
            let tmp = name
                .to_string()
                .trim_start_matches("indi_")
                .replace("_", " ")
                .to_string();
            content.push((tmp, format!("/usr/bin/{}", &name), false));
        }
    }

    content
}

pub fn start_indi(bins: Vec<String>) -> std::io::Result<std::process::Child> {
    let handle = Command::new("indiserver").args(&bins).spawn();

    handle
}

pub fn ensure_config_exists() {
    let full_config_path = dirs::config_dir().unwrap().join(&CONFIG_FOLDER);

    if !Path::new(&full_config_path).exists() {
        std::fs::create_dir(&full_config_path).unwrap();
    }

    let full_config_file_path = &full_config_path.join(&CONFIG_FILENAME);

    if !Path::new(&full_config_file_path).exists() {
        std::fs::File::create(&full_config_file_path).unwrap();

        // Dump a basic config
        let mut config = Ini::new();
        config.read(String::from(
            "[indiserver]\nautostart = false\ndrivers = \n",
        )).unwrap();
        config.write(&full_config_file_path).unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    raw_config: Ini,
}

impl Config {
    pub fn new() -> Config {
        let conf: Ini = read_config();
        Config::validate(&conf);
        Config {
            raw_config: read_config(),
        }
    }

    fn validate(conf: &Ini) {
        match &conf.get("indiserver", "autostart").unwrap()[..] {
            "true" | "false" => (),
            s => panic!(
                "The autostart value must be either `true` or `false`, found: {}",
                s
            ),
        }

        match conf.get("indiserver", "drivers") {
            None => (),
            _ => (),
        }
    }

    pub fn autostart(&self) -> bool {
        self.raw_config
            .getbool("indiserver", "autostart")
            .unwrap()
            .unwrap()
    }

    pub fn drivers(&self) -> Vec<String> {
        match self.raw_config.get("indiserver", "drivers") {
            None => Vec::new(),
            s => {
                let drivers_str = s.unwrap();
                let drivers = &drivers_str.split(",").collect::<Vec<&str>>();
                let mut result = vec![];

                for driver in drivers {
                    result.push(driver.to_string())
                }

                result
            }
        }
    }

    pub fn add_drivers_to_config(&mut self, drivers: &Vec<String>) {
        let drivers_str = drivers.join(",");
        self.raw_config
            .setstr("indiserver", "drivers", Some(&drivers_str.to_owned()));

        let path = dirs::config_dir()
            .unwrap()
            .join(&CONFIG_FOLDER)
            .join(&CONFIG_FILENAME);
        self.raw_config.write(&path).unwrap();
    }
}

fn config_file_path() -> PathBuf {
    let conf_dir = dirs::config_dir().unwrap();
    conf_dir.join(&CONFIG_FOLDER).join(&CONFIG_FILENAME)
}

/// Convenient entry point to read the configuration file, the configuration
/// is parsed and dumped into a HashMap which is then returned to the user so values can be fetched.
fn read_config() -> Ini {
    let mut config = Ini::new();
    config.load(&config_file_path()).unwrap();
    config
}
