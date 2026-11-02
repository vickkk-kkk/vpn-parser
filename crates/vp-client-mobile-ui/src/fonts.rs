//! Rendering-only bundled font integration. No system font discovery or I/O.
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const FONT_NAME: &str = "PrivateVP Sans SC Regular";
const FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/PrivateVPSansSC-Regular.ttf");
const LICENSE: &str = include_str!("../../../assets/fonts/OFL.txt");

/// Called once per MobileApp creation, before its first layout. Keep egui's
/// default fonts first; this face supplies missing glyphs in both UI families.
/// No global Once is used: a later egui Context also needs its own font setup.
pub(crate) fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(FONT_BYTES)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(FONT_NAME.to_owned());
    }
    ctx.set_fonts(fonts);
}

pub(crate) fn license_details(ui: &mut egui::Ui) {
    ui.collapsing("Bundled font and full license", |ui| {
        ui.label("PrivateVP Sans SC Regular · weight 400 · SIL Open Font License 1.1");
        ui.label("Derived from Noto Sans SC; static full-coverage instance, renamed PrivateVP Sans SC. No subsetting.");
        ui.label("Simplified-Chinese-oriented coverage, not all CJK or emoji. Locale-specific Japanese glyph forms are not guaranteed. Missing characters may still display as boxes.");
        ui.label("Font provenance and reproducible derivation recipe are distributed with the source in assets/fonts/provenance.json and assets/fonts/derive_font.py.");
        ui.separator();
        // Render the entire bundled license and copyright notice, without truncation.
        // The enclosing About page scrolls; this is plain text, not a web request.
        ui.label(LICENSE);
    });
}
