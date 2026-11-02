plugins { id("com.android.application") }
android {
    namespace = "org.vpnparser.client"
    compileSdk = 35
    defaultConfig {
        applicationId = "org.vpnparser.client"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0-dev"
        ndk { abiFilters += "arm64-v8a" }
    }
    // Supply libvp_client_android.so explicitly after the reviewed Rust target build.
    // No automatic Rust build, download, SDK install, or host-network action.
    sourceSets["main"].jniLibs.srcDir("src/main/jniLibs")
}
