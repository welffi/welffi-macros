use crate::accounts::{self, Server};
use crate::app::App;
use crate::links;
use crate::rejoin;
use crate::theme::{DANGER, FAINT};
use crate::widgets::{self, Kind};
use eframe::egui::{self, CursorIcon, RichText};

#[derive(Default)]
pub struct AddServer {
    open: bool,
    name: String,
    link: String,
    error: String,
}

type Options = Vec<(String, String)>;

fn dropdown(ui: &mut egui::Ui, id: &str, current: &str, options: &Options) -> Option<String> {
    let text = options.iter().find(|(value, _)| value == current).map_or("None", |(_, label)| label.as_str());
    let mut chosen = None;
    egui::ComboBox::from_id_salt(id).width(ui.available_width()).selected_text(text).show_ui(ui, |ui| {
        if ui.selectable_label(current.is_empty(), "None").clicked() {
            chosen = Some(String::new());
        }
        for (value, label) in options {
            if ui.selectable_label(value == current, label).clicked() {
                chosen = Some(value.clone());
            }
        }
    });
    chosen.filter(|value| value != current)
}

fn server_label(server: &Server) -> String {
    if server.name.trim().is_empty() { server.link.clone() } else { server.name.clone() }
}

impl App {
    pub fn refresh_rejoin(&mut self) {
        self.accounts = accounts::load_accounts();
        self.servers = rejoin::all_servers(&self.settings.servers, accounts::load_servers());
        self.apply_rejoin();
    }

    fn apply_rejoin(&mut self) {
        self.ctd.rejoin = rejoin::target(&self.settings.rejoin_account, &self.settings.rejoin_server, &self.accounts);
    }

    fn set_rejoin_account(&mut self, id: String) {
        self.settings.rejoin_account = id;
        self.settings.save();
        self.apply_rejoin();
    }

    fn set_rejoin_server(&mut self, link: String) {
        self.settings.rejoin_server = link;
        self.settings.save();
        self.apply_rejoin();
    }

    fn submit_server(&mut self) {
        let link = self.add_server.link.trim().to_string();
        if !links::is_private_link(&link) {
            self.add_server.error = "That isn't a private server link".to_string();
            return;
        }
        if !self.servers.iter().any(|server| server.link == link) {
            let name = self.add_server.name.trim().to_string();
            self.settings.servers.push(Server { name, link: link.clone() });
            self.settings.save();
            self.servers = rejoin::all_servers(&self.settings.servers, accounts::load_servers());
        }
        self.set_rejoin_server(link);
        self.add_server = AddServer::default();
    }

    pub fn rejoin_card(&mut self, ui: &mut egui::Ui) {
        widgets::section(ui, "REJOIN");
        let accounts: Options = self.accounts.iter().map(|account| (account.id.clone(), account.name.clone())).collect();
        let servers: Options = self.servers.iter().map(|server| (server.link.clone(), server_label(server))).collect();
        let idle = !self.ctd.running();
        ui.add_enabled_ui(idle, |ui| {
            ui.label(RichText::new("Account").size(11.5).color(FAINT));
            if let Some(id) = dropdown(ui, "rejoin_account", &self.settings.rejoin_account, &accounts) {
                self.set_rejoin_account(id);
            }
            ui.add_space(4.0);
            ui.label(RichText::new("Private server").size(11.5).color(FAINT));
            if let Some(link) = dropdown(ui, "rejoin_server", &self.settings.rejoin_server, &servers) {
                self.set_rejoin_server(link);
            }
            ui.add_space(4.0);
            self.add_server_form(ui);
        });
        if self.accounts.is_empty() {
            ui.label(RichText::new("No accounts found - add one in welffi accounts").size(11.0).color(FAINT));
        }
    }

    fn add_server_form(&mut self, ui: &mut egui::Ui) {
        if !self.add_server.open {
            let add = egui::Label::new(RichText::new("+ Add server").size(11.5).color(FAINT)).sense(egui::Sense::click());
            if ui.add(add).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                self.add_server.open = true;
            }
            return;
        }
        let form = &mut self.add_server;
        ui.add(egui::TextEdit::singleline(&mut form.name).hint_text("Name").desired_width(f32::INFINITY));
        ui.add(egui::TextEdit::singleline(&mut form.link).hint_text("Private server link").desired_width(f32::INFINITY));
        if !form.error.is_empty() {
            ui.label(RichText::new(&form.error).size(11.0).color(DANGER));
        }
        match widgets::button_row(ui, &[("Add", Kind::Accent), ("Cancel", Kind::Plain)], 28.0) {
            Some(0) => self.submit_server(),
            Some(_) => self.add_server = AddServer::default(),
            None => {}
        }
    }
}
