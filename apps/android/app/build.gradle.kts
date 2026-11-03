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

// Fail before packaging if the separately built arm64 library is missing or
// has the wrong basic ELF architecture. This is not a link or runtime test.
val verifyNativeLibrary by tasks.registering {
    val library = layout.projectDirectory.file("src/main/jniLibs/arm64-v8a/libvp_client_android.so")
    doLast {
        val file = library.asFile
        require(file.isFile && file.length() >= 64L) { "Missing arm64 libvp_client_android.so; build it before packaging." }
        val header = file.inputStream().use { it.readNBytes(20) }
        require(header.size == 20 && header[0] == 0x7f.toByte() && header[1] == 'E'.code.toByte()
            && header[2] == 'L'.code.toByte() && header[3] == 'F'.code.toByte()
            && header[4] == 2.toByte() && header[5] == 1.toByte()
            && header[16] == 3.toByte() && header[17] == 0.toByte()
            && header[18] == 0xb7.toByte() && header[19] == 0.toByte()) {
            "libvp_client_android.so must be a little-endian ELF64 AArch64 library."
        }
    }
}
tasks.named("preBuild") { dependsOn(verifyNativeLibrary) }
