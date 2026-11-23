plugins { id("com.android.application") }
android {
    namespace = "org.crysirror.player"
    compileSdk = 36
    defaultConfig {
        applicationId = "org.crysirror.player"
        minSdk = 28
        targetSdk = 36
        versionCode = 2
        versionName = "0.2.0"
        testInstrumentationRunner = "org.crysirror.player.NativeHostInstrumentation"
        ndk { abiFilters += (project.findProperty("crysirrorAbis") as String? ?: "arm64-v8a").split(',') }
    }
    // gradle :app:connectedReleaseAndroidTest -PcrysirrorHostTest=true
    testBuildType = "release"
    buildTypes { release {
        isMinifyEnabled = false
        // Opt-in local instrumentation only; distributable Release stays unsigned.
        if (project.findProperty("crysirrorHostTest") == "true") {
            signingConfig = signingConfigs.getByName("debug")
        }
    } }
    sourceSets { getByName("main") { jniLibs.srcDirs("../../../target/platform/android/jniLibs") } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
}
