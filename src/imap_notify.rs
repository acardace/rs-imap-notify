use crate::config::Config;
use crate::dbus_notify;
use imap;
use imap::types::Seq;
use log::{error, info, warn};
use mailparse::*;
use native_tls::{TlsConnector, TlsStream};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

pub fn connect(config: &Config) -> Result<imap::Session<TlsStream<TcpStream>>, String> {
    let tls = TlsConnector::builder()
        .build()
        .map_err(|err| err.to_string())?;

    let client = imap::connect((config.domain.as_str(), 993), config.domain.as_str(), &tls)
        .map_err(|err| err.to_string())?;

    client
        .login(&config.user, &config.password)
        .map_err(|e| e.0.to_string())
}

pub fn fetch_subject<T>(
    imap_session: &mut imap::Session<T>,
    unseen: &HashSet<Seq>,
) -> Result<HashSet<String>, String>
where
    T: Read + Write,
{
    let mut subjects = HashSet::new();
    let mut sequence: String = unseen.iter().map(|seq| format!("{},", seq)).collect();
    sequence.pop();

    match imap_session.fetch(sequence, "RFC822") {
        Ok(v) if v.len() > 0 => {
            for x in &v {
                let body = x.body().unwrap();

                subjects.insert(match parse_mail(body) {
                    Ok(mail) => {
                        format!("{}", mail.get_headers().get_first_value("Subject").unwrap())
                    }
                    Err(e) => return Err(format!("{}", e)),
                });
            }
            Ok(subjects)
        }
        Ok(_) => Err("no new messages found".to_owned()),
        Err(e) => Err(format!("{}", e)),
    }
}

pub fn list_unseen<T>(mut imap_session: &mut imap::Session<T>, mbox: &str)
where
    T: Read + Write,
{
    match imap_session.search("UNSEEN") {
        Ok(unseen) if unseen.len() > 0 => {
            let msgs = match fetch_subject(&mut imap_session, &unseen) {
                Ok(s) => s,
                Err(e) => {
                    warn!("{}", e);
                    let mut set = HashSet::with_capacity(1);
                    set.insert(format!("unread {}", unseen.len()));
                    set
                }
            };

            match dbus_notify::DBusNotifier::new() {
                Ok(notifier) => {
                    if let Err(e) = notifier.notify(
                        "imap-notify",
                        &format!("📫 {} ({})", mbox, unseen.len()),
                        &msgs.iter().nth(0).unwrap(),
                    ) {
                        error!("{}", e);
                    }
                }
                Err(e) => error!("{}", e),
            }

            info!("{} => unread {}", mbox, unseen.len());
            for msg in &msgs {
                info!("{}", msg);
            }
        }
        _ => info!("{} => no unread messages", mbox),
    }
}

pub fn notify_idle(config: Arc<Config>, mbox: &str) -> Result<(), String> {
    'connect: loop {
        let mut imap_session = connect(&config)?;

        imap_session.examine(&mbox).map_err(|err| err.to_string())?;

        list_unseen(&mut imap_session, mbox);

        if let Some(false) = config.wait_idle {
            imap_session.logout().map_err(|err| err.to_string())?;
            return Ok(());
        } else {
            loop {
                info!("Waiting for new messages on maibox {}", mbox);

                let idle = match imap_session.idle() {
                    Ok(i) => i,
                    Err(e) => {
                        error!("{}", e);
                        continue 'connect;
                    }
                };

                if let Err(_) = idle.wait_keepalive() {
                    continue 'connect;
                }

                list_unseen(&mut imap_session, mbox);
            }
        }
    }
}
