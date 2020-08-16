use clap;
use libc;
use serde::Deserialize;
use serde_yaml;
use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::ops::{Deref, DerefMut};

const CONFIG_FILE: &'static str = ".imap-notify.yml";

#[derive(Debug, Deserialize)]
pub struct ConfigData {
    pub domain: String,
    pub user: String,
    pub password: String,
    pub wait_idle: Option<bool>,
    pub mbox: Vec<String>,
}

#[derive(Debug)]
pub struct Config(Box<ConfigData>);

fn isatty() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

impl Config {
    pub fn new(matches: clap::ArgMatches) -> Result<Config, String> {
        let config = {
            let path = if let Some(c) = matches.value_of("CONFIG") {
                Some(c.to_owned())
            } else {
                match env::var("HOME") {
                    Ok(val) => Some(format!("{}/{}", &val, &CONFIG_FILE)),
                    Err(_) => None,
                }
            };

            if let Some(path) = path {
                if let Ok(file) = File::open(path) {
                    match serde_yaml::from_reader(file) {
                        Ok(c) => Some(Config(Box::new(c))),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(mut config) = config {
            if let Some(domain) = matches.value_of("DOMAIN") {
                config.0.domain = domain.to_owned()
            }
            if let Some(user) = matches.value_of("USER") {
                config.0.user = user.to_owned()
            }
            if let Some(password) = matches.value_of("PASSWD") {
                config.0.password = password.to_owned()
            }
            if matches.is_present("MONITOR") {
                config.0.wait_idle = Some(true)
            } else if isatty() {
                config.0.wait_idle = Some(false);
            }
            if let Some(mbox) = matches.value_of("MBOX") {
                println!("{}", mbox);
                config.0.mbox = mbox.split(' ').map(|s| s.to_owned()).collect();
            }

            Ok(config)
        } else {
            Ok(Config(Box::new(ConfigData {
                domain: matches.value_of("DOMAIN").unwrap().to_owned(),
                user: matches.value_of("USER").unwrap().to_owned(),
                password: matches.value_of("PASSWORD").unwrap().to_owned(),
                wait_idle: {
                    if matches.is_present("MONITOR") {
                        Some(true)
                    } else if isatty() {
                        Some(false)
                    } else {
                        None
                    }
                },
                mbox: {
                    if let Some(mbox) = matches.value_of("MBOX") {
                        mbox.split(',').map(|s| s.to_owned()).collect()
                    } else {
                        vec!["INBOX".to_owned()]
                    }
                },
            })))
        }
    }
}

impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Config {
    type Target = ConfigData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
