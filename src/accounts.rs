use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub tracker: String,
    pub sealed_cookie: String,
}

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Server {
    #[serde(alias = "Name")]
    pub name: String,
    #[serde(alias = "Link")]
    pub link: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
struct AccountsSettings {
    servers: Vec<Server>,
}

fn root() -> PathBuf {
    if let Some(dir) = std::env::var_os("WACCOUNTS_DATA") {
        return PathBuf::from(dir);
    }
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("welffi accounts")
}

fn read<T: serde::de::DeserializeOwned + Default>(file: &str) -> T {
    let text = std::fs::read_to_string(root().join(file)).unwrap_or_default();
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn load_accounts() -> Vec<Account> {
    read("accounts.json")
}

pub fn load_servers() -> Vec<Server> {
    read::<AccountsSettings>("settings.json").servers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_accounts_shape() {
        let accounts: Vec<Account> = serde_json::from_str(r#"[{"Id":"1","Name":"a","Tracker":"2","SealedCookie":"S"}]"#).unwrap();
        assert_eq!((accounts[0].id.as_str(), accounts[0].tracker.as_str(), accounts[0].sealed_cookie.as_str()), ("1", "2", "S"));
        let settings: AccountsSettings = serde_json::from_str(r#"{"Servers":[{"Name":"n","Link":"l"}],"LaunchGapMs":100}"#).unwrap();
        assert_eq!(settings.servers, [Server { name: "n".into(), link: "l".into() }]);
    }

    #[test]
    fn own_shape_round_trips() {
        let server = Server { name: "n".into(), link: "l".into() };
        let text = serde_json::to_string(&server).unwrap();
        assert_eq!(text, r#"{"name":"n","link":"l"}"#);
        assert_eq!(serde_json::from_str::<Server>(&text).unwrap(), server);
    }
}
