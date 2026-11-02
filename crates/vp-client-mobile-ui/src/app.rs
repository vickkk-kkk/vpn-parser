use crate::{
    editor::Editor,
    fonts,
    import::{display_text, ImportState},
    preview::{ConfigPreview, ENGINES, TEMPLATE_NOTICE},
    theme,
};
use eframe::egui::{self, RichText};
use vp_client_core::config::{RoutingMode, RuleAction, RuleMatcher, TrafficRule};
use vp_client_core::{
    geo::{BytesGeoProvider, GeoUnavailable},
    parser::NodeMetadata,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Nodes,
    Routing,
    Dns,
    About,
    Configuration,
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
    imports: ImportState,
    import_text: String,
    metadata: Box<dyn FnMut() -> NodeMetadata>,
    confirm_clear_imports: bool,
    config_preview: ConfigPreview,
}
impl MobileApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        platform: &'static str,
        metadata: Box<dyn FnMut() -> NodeMetadata>,
    ) -> Self {
        fonts::install(&cc.egui_ctx);
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
            page: Page::Nodes,
            dark: false,
            rule_domain: String::new(),
            message: "",
            confirm_clear_imports: false,
            config_preview: ConfigPreview::default(),
            imports: ImportState::default(),
            import_text: String::new(),
            metadata,
        }
    }

    fn nodes(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import subscription content");
        ui.label("Paste URI lists, base64 content or supported YAML/JSON. No URL fetching. Memory only; closing the process loses imported nodes.");
        ui.weak(
            "Content can contain credentials. It is not logged, saved or sent by this importer.",
        );
        let changed = ui
            .add(
                egui::TextEdit::multiline(&mut self.import_text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5),
            )
            .changed();
        if changed {
            self.config_preview.invalidate();
            if let Err(error) = self.imports.set_input(&self.import_text) {
                self.import_text.clear();
                self.message = error.message();
            } else {
                self.message = "Input changed; preview again before confirming.";
            }
        }
        if ui.button("Preview content").clicked() {
            // Missing assets are explicitly missing, never a fabricated empty database.
            let geo = BytesGeoProvider {
                site: Err(GeoUnavailable::Missing),
                ip: Err(GeoUnavailable::Missing),
            };
            self.message = match self.imports.prepare(&mut self.metadata, &geo) {
                Ok(()) => "Preview only. Unsupported or malformed entries may be omitted.",
                Err(error) => error.message(),
            };
        }
        if ui.button("Cancel / clear paste").clicked() {
            self.config_preview.invalidate();
            self.imports.cancel();
            self.import_text.clear();
            self.message = "Paste and preview cleared; imported nodes unchanged.";
        }
        if let Some(preview) = self.imports.preview() {
            ui.separator();
            ui.label(format!(
                "{} recognized nodes · {} rule drafts",
                preview.nodes().len(),
                preview.rules().len()
            ));
            ui.label(
                "Geo assets not supplied. Rules are unverified and will not be applied to traffic.",
            );
            for warning in preview.warnings().iter().take(12) {
                ui.label(format!(
                    "Rule {}: {}",
                    warning.rule_index.saturating_add(1),
                    warning.message()
                ));
            }
            if preview.warnings().len() > 12 {
                ui.label("Additional geo warnings omitted from this view.");
            }
            for node in preview.nodes().iter().take(20) {
                ui.label(format!(
                    "{} · {} · {}:{} · Not tested",
                    display_text(&node.name),
                    display_text(&node.protocol),
                    display_text(&node.server),
                    node.port
                ));
            }
            if preview.nodes().len() > 20 {
                ui.label("Preview shows the first 20 nodes.");
            }
        }
        if ui
            .add_enabled(
                self.imports.preview().is_some(),
                egui::Button::new("Confirm import to memory"),
            )
            .clicked()
        {
            self.config_preview.invalidate();
            self.message = match self.imports.confirm() {
                Ok(0) => {
                    self.import_text.clear();
                    "No new nodes; duplicates already exist. Rule drafts retained locally."
                }
                Ok(_) => {
                    self.import_text.clear();
                    "Nodes imported to memory; no connection or latency test started."
                }
                Err(error) => error.message(),
            };
        }
        ui.separator();
        ui.label(format!(
            "Imported: {} nodes · {} unapplied rule drafts",
            self.imports.nodes().len(),
            self.imports.rules().len()
        ));
        egui::ScrollArea::vertical()
            .id_salt("imported-nodes")
            .max_height(260.0)
            .show(ui, |ui| {
                for node in self.imports.nodes() {
                    ui.label(format!(
                        "{} · {} · Not tested",
                        display_text(&node.name),
                        display_text(&node.protocol)
                    ));
                }
            });
    }

    fn configuration(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configuration preview");
        ui.label(TEMPLATE_NOTICE);
        ui.weak("Local-port template fields are retained by the upstream compilers. No port is opened. Controller authentication is not provisioned; do not run this draft.");
        egui::ComboBox::from_id_salt("preview-engine")
            .selected_text(
                self.config_preview
                    .engine()
                    .map(|e| e.display_name())
                    .unwrap_or("Choose engine"),
            )
            .show_ui(ui, |ui| {
                for engine in ENGINES {
                    if ui
                        .selectable_label(
                            self.config_preview.engine() == Some(engine),
                            engine.display_name(),
                        )
                        .clicked()
                    {
                        self.config_preview.select_engine(engine);
                    }
                }
            });
        let selected_label = self
            .imports
            .nodes()
            .iter()
            .find(|n| Some(n.id.as_str()) == self.config_preview.node_id())
            .map(|n| display_text(&n.name))
            .unwrap_or_else(|| "Choose imported node".into());
        egui::ComboBox::from_id_salt("preview-node")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                for node in self.imports.nodes() {
                    ui.push_id(&node.id, |ui| {
                        if ui
                            .selectable_label(
                                self.config_preview.node_id() == Some(node.id.as_str()),
                                format!(
                                    "{} · {}",
                                    display_text(&node.name),
                                    display_text(&node.protocol)
                                ),
                            )
                            .clicked()
                        {
                            self.config_preview.select_node(Some(node.id.clone()));
                        }
                    });
                }
            });
        ui.label(format!(
            "Routing: {:?}. Manual rules take precedence over imported rule drafts.",
            self.editor.mode
        ));
        ui.label("Global / Direct modes do not compile custom rule drafts. Rule mode adds upstream presets; unsupported rules (including Shoes QUIC rules) are errors, not an automatic mode change.");
        ui.label("Geo inventories and engine asset paths are missing. No rule database is fabricated or downloaded.");
        if ui.button("Compile draft for selected engine").clicked() {
            self.config_preview.compile(
                self.imports.nodes(),
                &self.editor.dns,
                &self.editor.nameservers,
                self.editor.mode,
                &self.editor.rules,
                self.imports.rules(),
            );
        }
        ui.label(self.config_preview.summary());
        if self.config_preview.inactive_rules() {
            ui.label("Custom rule drafts are inactive in this routing mode.");
        }
        for diagnostic in self.config_preview.diagnostics() {
            let rule = diagnostic
                .rule_index
                .map(|i| format!(" · rule {}", i.saturating_add(1)))
                .unwrap_or_default();
            // Compiler diagnostics are credential-free; additionally bound/control-filter display.
            ui.label(format!(
                "{:?}{}: {}",
                diagnostic.severity,
                rule,
                display_text(&diagnostic.message)
            ));
        }
        ui.separator();
        ui.label("Full configuration contains credentials. Revealing it makes secrets visible on screen.");
        if ui
            .add_enabled(
                self.config_preview.has_artifact(),
                egui::Button::new("Reveal full configuration (contains secrets)"),
            )
            .clicked()
        {
            self.config_preview.reveal();
        }
        if ui.button("Hide configuration").clicked() {
            self.config_preview.hide();
        }
        if let Some(text) = self.config_preview.revealed() {
            egui::ScrollArea::both()
                .id_salt("revealed-config")
                .max_height(300.0)
                .show(ui, |ui| {
                    ui.monospace(text);
                });
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
            if ui
                .selectable_value(&mut self.editor.mode, mode, label)
                .changed()
            {
                self.config_preview.invalidate();
            }
        }
        ui.separator();
        ui.label(RichText::new("Direct domain rules").strong());
        ui.label("Domain suffix, for example example.invalid");
        if ui
            .add(egui::TextEdit::singleline(&mut self.rule_domain).desired_width(f32::INFINITY))
            .changed()
        {
            self.config_preview.invalidate();
        }
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
                self.config_preview.invalidate();
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
            self.config_preview.invalidate();
            self.editor.rules.remove(index);
        }
        if self.editor.rules.is_empty() {
            ui.weak("No custom rules in this draft.");
        }
    }

    fn dns(&mut self, ui: &mut egui::Ui) {
        ui.heading("DNS draft");
        ui.label("One IP or HTTPS resolver per line. Validation uses the upstream DNS configuration model; it does not test reachability or security.");
        if ui
            .add(
                egui::TextEdit::multiline(&mut self.editor.nameservers)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5),
            )
            .changed()
        {
            self.config_preview.invalidate();
        }
        if ui
            .add_sized(
                [ui.available_width(), 48.0],
                egui::Button::new("Validate draft"),
            )
            .clicked()
        {
            self.config_preview.invalidate();
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
            ui.columns(5, |columns| {
                for (column, (page, title)) in columns.iter_mut().zip([
                    (Page::Nodes, "Nodes"),
                    (Page::Routing, "Routing"),
                    (Page::Dns, "DNS"),
                    (Page::Configuration, "Config"),
                    (Page::About, "About"),
                ]) {
                    if column
                        .add_sized(
                            [column.available_width(), 48.0],
                            egui::Button::new(title).selected(self.page == page),
                        )
                        .clicked()
                    {
                        self.config_preview.hide();
                        self.page = page;
                        self.confirm_clear_imports = false;
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
                    Page::Nodes => self.nodes(ui),
                    Page::Routing => self.routing(ui),
                    Page::Dns => self.dns(ui),
                    Page::Configuration => self.configuration(ui),
                    Page::About => {
                        ui.heading("Mobile configuration shell");
                        ui.label(format!("{} · egui · shared Rust core", self.platform));
                        ui.label("Built from the VPN Parser fork, reusing its palette, DNS validator and routing models.");
                        ui.label("Not connected. Paste import is memory-only. Subscription fetching, secure storage and native VPN lifecycle are not integrated yet. Drafts disappear when the process closes.");
                        if ui.checkbox(&mut self.dark, "Dark appearance").changed() {
                            theme::apply(ctx, theme::DEFAULT_THEME, self.dark);
                            ctx.style_mut(|style| style.spacing.interact_size.y = 48.0);
                        }
                        if ui.button("Reset Routing / DNS draft").clicked() {
                            self.config_preview.invalidate();
                            self.editor = Editor::default(); self.rule_domain.clear();
                            self.message = "Routing / DNS draft reset. Imported data unchanged.";
                        }
                        ui.separator();
                        if ui.button("Clear all imported data...").clicked() {
                            self.confirm_clear_imports = true;
                        }
                        if self.confirm_clear_imports {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.label("Remove all imported nodes, imported rule drafts, preview and pasted content from this app state?");
                                ui.weak("Routing / DNS drafts are unchanged. This is not secure memory erasure or clipboard clearing.");
                                if ui.button("Confirm: clear imported data").clicked() {
                                    self.config_preview.invalidate();
                                    self.config_preview.select_node(None);
                                    self.imports = ImportState::default();
                                    self.import_text.clear();
                                    self.confirm_clear_imports = false;
                                    self.message = "Imported nodes, rules, preview and paste cleared from app state.";
                                }
                                if ui.button("Cancel clearing").clicked() {
                                    self.confirm_clear_imports = false;
                                    self.message = "Clear canceled. Imported data unchanged.";
                                }
                            });
                        }
                        fonts::license_details(ui);
                        ui.label("Upstream: mateo-ki/vpn-parser · MIT license");
                    }
                }
                if !self.message.is_empty() { ui.separator(); ui.label(self.message); }
                ui.add_space(20.0);
            });
        });
    }
}
