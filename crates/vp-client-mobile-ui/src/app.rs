use crate::{
    editor::Editor,
    fonts,
    import::{display_text, ImportState},
    preview::{ConfigPreview, ENGINES, TEMPLATE_NOTICE},
    runtime::{parse_dns, RuntimeError, RuntimeHost, RuntimeObservation},
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
    Connection,
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
    runtime: Option<Box<dyn RuntimeHost>>,
    runtime_node: Option<String>,
    runtime_bootstrap: String,
    runtime_dns: String,
    runtime_prepared: bool,
    runtime_message: &'static str,
    runtime_observation: RuntimeObservation,
    runtime_last_observed: Option<std::time::Instant>,
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
            runtime: None,
            runtime_node: None,
            runtime_bootstrap: String::new(),
            runtime_dns: String::new(),
            runtime_prepared: false,
            runtime_message: "尚未准备连接。",
            runtime_observation: RuntimeObservation::Unavailable,
            runtime_last_observed: None,
        }
    }

    pub fn with_runtime(mut self, runtime: Box<dyn RuntimeHost>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    fn clear_runtime_draft(&mut self) {
        self.runtime_prepared = false;
        self.runtime_message = "参数已变化，请重新检查；运行中的 VPN 不会自动停止。";
        if let Some(runtime) = self.runtime.as_mut() { runtime.clear_prepared(); }
    }

    fn runtime_error(error: RuntimeError) -> &'static str {
        match error {
            RuntimeError::Unavailable => "原生适配组件不可用。",
            RuntimeError::Unsupported => "当前内核不支持此节点或配置。",
            RuntimeError::InvalidInput => "节点或 DNS 输入无效。",
            RuntimeError::PermissionRequired => "需要先获得系统 VPN 授权。",
            RuntimeError::Failed => "操作失败；请检查原生状态。",
        }
    }

    fn connection(&mut self, ui: &mut egui::Ui) {
        ui.heading("Android VPN 实验连接");
        ui.label("仅 vp_core；其他内核尚未接入。请在 Routing 选择 Global proxy；规则草稿不会应用。连接和 DNS 泄漏尚未验收。");
        if self.runtime.is_none() { ui.label("原生组件未绑定。"); return; }
        let selected = self.imports.nodes().iter().find(|n| Some(n.id.as_str()) == self.runtime_node.as_deref());
        let label = selected.map(|n| display_text(&n.name)).unwrap_or_else(|| "选择节点".into());
        let mut selected_id = self.runtime_node.clone();
        egui::ComboBox::from_id_salt("runtime-node").selected_text(label).show_ui(ui, |ui| {
            for node in self.imports.nodes() {
                ui.selectable_value(&mut selected_id, Some(node.id.clone()), display_text(&node.name));
            }
        });
        if selected_id != self.runtime_node { self.runtime_node = selected_id; self.clear_runtime_draft(); }
        ui.label("节点域名引导 DNS（直接访问，输入 IP:端口，每行一个）：");
        let bootstrap_changed = ui.text_edit_multiline(&mut self.runtime_bootstrap).changed();
        ui.label("用户 DNS（通过所选代理，输入 IP:端口，每行一个）：");
        let dns_changed = ui.text_edit_multiline(&mut self.runtime_dns).changed();
        if bootstrap_changed || dns_changed { self.clear_runtime_draft(); }
        if ui.button("检查连接参数").clicked() {
            self.clear_runtime_draft();
            let node = self.imports.nodes().iter().find(|n| Some(n.id.as_str()) == self.runtime_node.as_deref());
            let result = if !matches!(self.editor.mode, RoutingMode::Global) {
                Err(RuntimeError::Unsupported)
            } else { match (node, parse_dns(&self.runtime_bootstrap), parse_dns(&self.runtime_dns), self.runtime.as_mut()) {
                (Some(node), Ok(bootstrap), Ok(dns), Some(runtime)) => {
                    if runtime.available() { runtime.prepare(&node.raw, &node.id, &bootstrap, &dns) }
                    else { Err(RuntimeError::Unavailable) }
                }
                _ => Err(RuntimeError::InvalidInput),
            }};
            self.runtime_message = match result {
                Ok(()) => { self.runtime_prepared = true; "参数通过预检；尚未请求授权或启动。" }
                Err(e) => Self::runtime_error(e),
            };
        }
        if ui.add_enabled(self.runtime_prepared, egui::Button::new("请求系统 VPN 授权")).clicked() {
            self.runtime_message = match self.runtime.as_mut().unwrap().request_consent() {
                Ok(()) => "已请求授权；请完成系统提示后手动启动。",
                Err(e) => Self::runtime_error(e),
            };
        }
        if ui.add_enabled(self.runtime_prepared, egui::Button::new("启动 vp_core")).clicked() {
            let present = self.imports.nodes().iter().any(|n| Some(n.id.as_str()) == self.runtime_node.as_deref());
            let host = self.runtime.as_mut().unwrap();
            self.runtime_message = if !matches!(self.editor.mode, RoutingMode::Global) { "当前仅支持全局代理，请重新检查。" }
                else if !present { "选中的节点已不存在，请重新检查。" }
                else if !host.consent_granted() { Self::runtime_error(RuntimeError::PermissionRequired) }
                else { match host.start() {
                    Ok(()) => "启动请求已提交；流量转发和 DNS 尚未验证。",
                    Err(e) => Self::runtime_error(e),
                }};
        }
        if ui.button("停止 VPN").clicked() {
            self.runtime_message = match self.runtime.as_mut().unwrap().stop() {
                Ok(()) => "停止请求已提交；请等待原生组件确认。",
                Err(e) => Self::runtime_error(e),
            };
        }
        ui.label(self.runtime_message);
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
        ui.label("Routing drafts do not change an active tunnel. The experimental Android host only accepts Global proxy; rule and direct modes remain unavailable.");
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
                self.clear_runtime_draft();
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
        if let Some(runtime) = self.runtime.as_mut() {
            if self.runtime_last_observed.map(|at| at.elapsed() >= std::time::Duration::from_secs(2)).unwrap_or(true) {
                self.runtime_observation = runtime.observe();
                self.runtime_last_observed = Some(std::time::Instant::now());
            }
            ctx.request_repaint_after(std::time::Duration::from_secs(2));
        }
        let runtime_label = match self.runtime_observation {
            RuntimeObservation::Unavailable => "原生组件不可用或无状态",
            RuntimeObservation::Idle => "原生组件报告空闲",
            RuntimeObservation::Starting => "正在启动，尚未验证",
            RuntimeObservation::Verifying => "正在验证，尚未确认连接",
            RuntimeObservation::ReportedActive => "内核报告活跃；转发和 DNS 未独立验证",
            RuntimeObservation::Stopping => "正在停止",
            RuntimeObservation::Failed => "原生组件报告失败",
            RuntimeObservation::Stale => "原生状态已过期，无法确认连接",
        };
        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("VPN Parser");
            ui.colored_label(
                theme::palette_for(theme::DEFAULT_THEME, self.dark).warning,
                if self.runtime.is_some() { runtime_label } else { "Tunnel unavailable · no native engine bound" },
            );
            if self.runtime.is_none() { ui.add_enabled(false, egui::Button::new("Connect unavailable")); }
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("navigation").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.columns(6, |columns| {
                for (column, (page, title)) in columns.iter_mut().zip([
                    (Page::Nodes, "Nodes"),
                    (Page::Routing, "Routing"),
                    (Page::Dns, "DNS"),
                    (Page::Configuration, "Config"),
                    (Page::Connection, "VPN"),
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
                    Page::Connection => self.connection(ui),
                    Page::About => {
                        ui.heading("Mobile configuration shell");
                        ui.label(format!("{} · egui · shared Rust core", self.platform));
                        ui.label("Built from the VPN Parser fork, reusing its palette, DNS validator and routing models.");
                        if self.runtime.is_some() {
                            ui.label("Experimental vp_core host bound. Import is memory-only; VPN and DNS behavior are not yet verified. Drafts disappear when the process closes.");
                        } else {
                            ui.label("Not connected. Paste import is memory-only. Subscription fetching, secure storage and native VPN lifecycle are not integrated yet. Drafts disappear when the process closes.");
                        }
                        if ui.checkbox(&mut self.dark, "Dark appearance").changed() {
                            theme::apply(ctx, theme::DEFAULT_THEME, self.dark);
                            ctx.style_mut(|style| style.spacing.interact_size.y = 48.0);
                        }
                        if ui.button("Reset Routing / DNS draft").clicked() {
                            self.config_preview.invalidate();
                            self.clear_runtime_draft();
                            self.runtime_bootstrap.clear(); self.runtime_dns.clear();
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
                                ui.weak("Routing / DNS drafts are unchanged. This does not stop an active VPN, securely erase memory, or clear the clipboard.");
                                if ui.button("Confirm: clear imported data").clicked() {
                                    self.clear_runtime_draft(); self.runtime_node = None;
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
