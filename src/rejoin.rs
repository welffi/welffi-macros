use crate::accounts::{Account, Server};

#[derive(Clone)]
pub struct Target {
    pub account: Account,
    pub link: String,
}

pub fn all_servers(own: &[Server], theirs: Vec<Server>) -> Vec<Server> {
    let mut all = own.to_vec();
    for server in theirs {
        if !all.iter().any(|known| known.link == server.link) {
            all.push(server);
        }
    }
    all
}

pub fn target(account_id: &str, link: &str, accounts: &[Account]) -> Option<Target> {
    if link.is_empty() {
        return None;
    }
    let account = accounts.iter().find(|account| account.id == account_id)?;
    Some(Target { account: account.clone(), link: link.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, link: &str) -> Server {
        Server { name: name.into(), link: link.into() }
    }

    fn account(id: &str) -> Account {
        Account { id: id.into(), ..Account::default() }
    }

    #[test]
    fn own_servers_come_first_and_links_are_not_repeated() {
        let all = all_servers(&[server("mine", "a")], vec![server("theirs", "a"), server("other", "b")]);
        assert_eq!(all, [server("mine", "a"), server("other", "b")]);
    }

    #[test]
    fn a_target_needs_a_link_and_a_known_account() {
        let accounts = [account("1"), account("2")];
        assert_eq!(target("2", "link", &accounts).map(|target| target.account.id), Some("2".to_string()));
        assert!(target("2", "", &accounts).is_none());
        assert!(target("9", "link", &accounts).is_none());
        assert!(target("", "link", &accounts).is_none());
    }
}
