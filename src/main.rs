mod config;
mod dbus_notify;
mod imap_notify;

use clap::{clap_app, crate_authors, crate_version};
use config::Config;
use log::error;
use simple_logger::SimpleLogger;
use std::file;
use std::line;
use std::process;
use std::sync::Arc;
use std::thread;

fn do_imap(config: Config) -> Result<(), String> {
    let mut handles = vec![];
    let mut imap_session = imap_notify::connect(&config)?;
    let arc_config = Arc::new(config);

    for mbox_name in arc_config.mbox.iter() {
        match imap_session.examine(mbox_name) {
            Ok(_) => {
                let mbox_clone = mbox_name.clone();
                let config_clone = arc_config.clone();

                let handle = thread::spawn(move || {
                    if let Err(e) = imap_notify::notify_idle(config_clone, &mbox_clone) {
                        error!("Error: {} {}", e, &mbox_clone);
                    }
                });
                handles.push(handle);
            }
            Err(e) => return Err(format!("{}:{} {}", file!(), line!(), e)),
        }
    }

    if let Err(e) = imap_session.logout() {
        Err(format!("{}:{} {}", file!(), line!(), e))
    } else {
        for h in handles {
            let _res = h.join();
        }
        Ok(())
    }
}

fn main() {
    let matches = clap_app!(imap_notify =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Monitor your IMAP mailbox")
        (@arg CONFIG: -c --config +takes_value conflicts_with[configuration]
         "Load config file, defaults to $(HOME)/.imap_notify")
        (@group configuration =>
            (@arg USER: -u --user +takes_value "IMAP user")
            (@arg PASSWD: -p --password +takes_value "IMAP password")
            (@arg DOMAIN: -d --domain +takes_value "IMAP domain")
        )
        (@arg MONITOR: -m --monitor "Monitor INBOX")
        (@arg MBOX: -x --mailbox +takes_value +multiple "Mailboxes to monitor")
    )
    .get_matches();

    match SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .init()
    {
        Ok(_) => {}
        Err(_) => eprintln!("Error: Could not init logger"),
    }

    let config = Config::new(matches).unwrap_or_else(|err| {
        error!("{}", err);
        process::exit(1);
    });

    if let Err(e) = do_imap(config) {
        error!("{}", e);
    }
}
