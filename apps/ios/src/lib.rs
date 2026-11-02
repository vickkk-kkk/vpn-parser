//! Called from the native process main, on the main thread, before UIApplicationMain.
//! winit owns UIApplication and its delegate; do not wrap this in SwiftUI/AppDelegate.
#![cfg(target_os = "ios")]

#[no_mangle]
pub extern "C" fn vp_client_ios_main() -> std::ffi::c_int {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        run_and_return: false,
        persist_window: false,
        ..Default::default()
    };
    match eframe::run_native(
        "VPN Parser",
        options,
        Box::new(|cc| Ok(Box::new(vp_client_mobile_ui::MobileApp::new(cc, "iOS")))),
    ) {
        Ok(()) => 0,
        Err(_) => 1, // No profile/credential data crosses the native diagnostic boundary.
    }
}
