use std::process::{Command, Stdio};

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
