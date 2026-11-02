use crate::{editor::Editor, theme};
use eframe::egui::{self, RichText};
use vp_client_core::config::{RoutingMode, RuleAction, RuleMatcher, TrafficRule};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Routing,
    Dns,
    About,
}

/// Mobile adaptation of the fork's configuration workflow and palette.
/// No connection state exists until an actual native backend is integrated.
pub struct MobileApp {
    platform: &'static str,
    editor: Editor,
    page: Page,
    dark: bool,
    rule_domain: String,
    message: &'static str,
}
impl MobileApp {
    pub fn new(cc: &eframe::CreationContext<'_>, platform: &'static str) -> Self {
        theme::apply(&cc.egui_ctx, theme::DEFAULT_THEME, false);
        cc.egui_ctx.style_mut(|style| {
            style.spacing.interact_size.y = 48.0;
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(17.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(17.0));
        });
        Self {
            platform,
            editor: Editor::default(),
            page: Page::Routing,
            dark: false,
            rule_domain: String::new(),
            message: "",
        }
    }

    fn routing(&mut self, ui: &mut egui::Ui) {
        ui.heading("Routing draft");
        ui.label("Uses the same routing types as the desktop client. Changes are not applied to device traffic.");
        for (mode, label) in [
            (RoutingMode::Rule, "Rules"),
            (RoutingMode::Global, "Global proxy"),
            (RoutingMode::Direct, "Direct"),
        ] {
            ui.selectable_value(&mut self.editor.mode, mode, label);
        }
        ui.separator();
        ui.label(RichText::new("Direct domain rules").strong());
        ui.label("Domain suffix, for example example.invalid");
        ui.add(egui::TextEdit::singleline(&mut self.rule_domain).desired_width(f32::INFINITY));
        if ui
            .add_sized(
                [ui.available_width(), 48.0],
                egui::Button::new("Add to draft"),
            )
            .clicked()
        {
            let value = self.rule_domain.trim();
            let rule = TrafficRule {
                matcher: RuleMatcher::DomainSuffix(value.into()),
                action: RuleAction::Direct,
            };
            if value.is_empty() || rule.to_mihomo_rule().is_err() {
                self.message = "Enter a non-empty domain without rule delimiters.";
            } else if self.editor.rules.contains(&rule) {
                self.message = "That rule is already in the draft.";
            } else if self.editor.rules.len() >= 256 {
                self.message = "Draft limit reached (256 rules). Remove a rule first.";
            } else {
                self.editor.rules.push(rule);
                self.rule_domain.clear();
                self.message = "Rule added locally; no traffic settings changed.";
            }
        }
        let mut remove = None;
        for (index, rule) in self.editor.rules.iter().enumerate() {
            ui.push_id(index, |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    if let RuleMatcher::DomainSuffix(domain) = &rule.matcher {
                        ui.label(domain);
                    }
                    if ui.button("Remove").clicked() {
                        remove = Some(index);
                    }
                });
            });
        }
        if let Some(index) = remove {
            self.editor.rules.remove(index);
        }
        if self.editor.rules.is_empty() {
            ui.weak("No custom rules in this draft.");
        }
    }

    fn dns(&mut self, ui: &mut egui::Ui) {
        ui.heading("DNS draft");
        ui.label("One IP or HTTPS resolver per line. Validation uses the upstream DNS configuration model; it does not test reachability or security.");
        ui.add(
            egui::TextEdit::multiline(&mut self.editor.nameservers)
                .desired_width(f32::INFINITY)
                .desired_rows(5),
        );
        if ui
            .add_sized(
                [ui.available_width(), 48.0],
                egui::Button::new("Validate draft"),
            )
            .clicked()
        {
            self.message = if self.editor.validate_dns_draft() {
                "Draft passes upstream validation. Device DNS is unchanged."
            } else {
                "Invalid DNS draft. Use at least one IP or HTTPS resolver."
            };
        }
        ui.separator();
        ui.label("Upstream node-hostname resolvers (not active):");
        for resolver in &self.editor.dns.proxy_server_nameserver {
            ui.label(resolver);
        }
        ui.weak("Upstream defaults include plaintext bootstrap resolvers. They are shown for transparency, not a no-leak guarantee.");
    }
}
impl eframe::App for MobileApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("VPN Parser");
            ui.colored_label(
                theme::palette_for(theme::DEFAULT_THEME, self.dark).warning,
                "Tunnel unavailable · no native engine bound",
            );
            ui.add_enabled(false, egui::Button::new("Connect unavailable"));
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("navigation").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.columns(3, |columns| {
                for (column, (page, title)) in columns.iter_mut().zip([
                    (Page::Routing, "Routing"),
                    (Page::Dns, "DNS"),
                    (Page::About, "About"),
                ]) {
                    if column
                        .add_sized(
                            [column.available_width(), 48.0],
                            egui::Button::new(title).selected(self.page == page),
                        )
                        .clicked()
                    {
                        self.page = page;
                        self.message = "";
                    }
                }
            });
            ui.add_space(8.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(620.0);
                ui.add_space(8.0);
                match self.page {
                    Page::Routing => self.routing(ui),
                    Page::Dns => self.dns(ui),
                    Page::About => {
                        ui.heading("Mobile configuration shell");
                        ui.label(format!("{} · egui · shared Rust core", self.platform));
                        ui.label("Built from the VPN Parser fork, reusing its palette, DNS validator and routing models.");
                        ui.label("Not connected. Node import, subscriptions, secure storage and native VPN lifecycle are not integrated yet. Drafts disappear when the process closes.");
                        if ui.checkbox(&mut self.dark, "Dark appearance").changed() {
                            theme::apply(ctx, theme::DEFAULT_THEME, self.dark);
                            ctx.style_mut(|style| style.spacing.interact_size.y = 48.0);
                        }
                        if ui.button("Reset local draft").clicked() {
                            self.editor = Editor::default(); self.rule_domain.clear();
                            self.message = "Local draft reset.";
                        }
                        ui.label("Upstream: mateo-ki/vpn-parser · MIT license");
                    }
                }
                if !self.message.is_empty() { ui.separator(); ui.label(self.message); }
                ui.add_space(20.0);
            });
        });
    }
}
