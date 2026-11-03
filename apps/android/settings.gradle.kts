pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
dependencyResolutionManagement { repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories { google(); mavenCentral() } }
rootProject.name = "VPNParserAndroid"
include(":app")
// The unified product repository supplies the optional native runtime module.
val runtimeRoot = providers.gradleProperty("vpEguiRuntimeRoot").orNull
if (runtimeRoot != null) {
    include(":vp-egui-runtime")
    project(":vp-egui-runtime").projectDir = file("$runtimeRoot/native/android/egui")
}
