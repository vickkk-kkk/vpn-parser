plugins { id("com.android.application") }
val nativeLibDirectory = providers.gradleProperty("vpNativeLibDir").orElse("src/main/jniLibs")
val runtimeEnabled = providers.gradleProperty("vpEguiRuntimeRoot").isPresent
providers.gradleProperty("vpBuildDir").orNull?.let { layout.buildDirectory.set(file(it)) }
android {
    namespace = "org.vpnparser.client"
    compileSdk = 35
    defaultConfig {
        applicationId = "org.vpnparser.client"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0-dev"
        manifestPlaceholders["vpHasCode"] = if (runtimeEnabled) "true" else "false"
        ndk { abiFilters += "arm64-v8a" }
    }
    // Supply libvp_client_android.so explicitly after the reviewed Rust target build.
    // No automatic Rust build, download, SDK install, or host-network action.
    sourceSets["main"].jniLibs.setSrcDirs(listOf(nativeLibDirectory.get()))
}
if (runtimeEnabled) {
    dependencies { add("implementation", project(":vp-egui-runtime")) }
}

// Fail before packaging if the separately built arm64 library is missing or
// has the wrong basic ELF architecture. This is not a link or runtime test.
val verifyNativeLibrary by tasks.registering {
    val names = mutableListOf("libvp_client_android.so")
    if (runtimeEnabled) names.addAll(listOf("libvp_android_host.so", "libvp_product_core_adapter.so"))
    doLast {
        for (name in names) {
            val file = file("${nativeLibDirectory.get()}/arm64-v8a/$name")
            require(file.isFile && file.length() >= 64L) { "Missing arm64 $name; build it before packaging." }
            val header = file.inputStream().use { it.readNBytes(20) }
            require(header.size == 20 && header[0] == 0x7f.toByte() && header[1] == 'E'.code.toByte()
                && header[2] == 'L'.code.toByte() && header[3] == 'F'.code.toByte()
                && header[4] == 2.toByte() && header[5] == 1.toByte()
                && header[16] == 3.toByte() && header[17] == 0.toByte()
                && header[18] == 0xb7.toByte() && header[19] == 0.toByte()) {
                "$name must be a little-endian ELF64 AArch64 library."
            }
        }
    }
}
tasks.named("preBuild") { dependsOn(verifyNativeLibrary) }
