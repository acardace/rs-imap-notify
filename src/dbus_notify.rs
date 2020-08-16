use dbus::arg;
use dbus::blocking::{Connection, Proxy};
use log::error;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;

struct DBusConnection(Connection);

impl Deref for DBusConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct DBusNotifier<'a> {
    proxy: Proxy<'a, DBusConnection>,
}

impl<'a> DBusNotifier<'a> {
    pub fn new() -> Result<DBusNotifier<'a>, &'static str> {
        let conn = match Connection::new_session() {
            Ok(conn) => DBusConnection(conn),
            Err(e) => {
                error!("{}", e);
                return Err("cannot create a new connection");
            }
        };

        Ok(DBusNotifier {
            proxy: Proxy::new(
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                Duration::from_millis(2000),
                conn,
            ),
        })
    }

    pub fn notify(&self, application: &str, title: &str, body: &str) -> Result<(), String> {
        let mut hints: HashMap<&str, arg::Variant<Box<dyn arg::RefArg>>> =
            HashMap::with_capacity(1);

        hints.insert(
            "category",
            arg::Variant(Box::new("email.arrived".to_string())),
        );

        match self.proxy.method_call(
            "org.freedesktop.Notifications",
            "Notify",
            (
                application,
                0 as u32,
                "",
                title,
                body,
                &[] as &[&str],
                hints,
                10000,
            ),
        ) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!(
                "{} {}",
                e.name()
                    .unwrap_or("Error while calling Notify on org.freedesktop.Notifications"),
                e.message().unwrap_or("")
            )),
        }
    }
}
