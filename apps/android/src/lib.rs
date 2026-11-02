//! Android entry only: never enter the desktop startup path.
#![cfg(target_os = "android")]

use winit::platform::android::activity::AndroidApp;

/// android-activity invokes this on its native event-loop thread.
#[no_mangle]
fn android_main(app: AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "VPN Parser",
        options,
        Box::new(|cc| Ok(Box::new(vp_client_mobile_ui::MobileApp::new(cc, "Android")))),
    ) {
        // Rendering failure is not a tunnel state transition. Never log profile data.
        eprintln!("Unable to start VPN Parser UI: {error}");
    }
}
