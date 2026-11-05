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
        ndk { abiFilters += (project.findProperty("crysirrorAbis") as String? ?: "arm64-v8a").split(',') }
    }
    buildTypes { release { isMinifyEnabled = false } }
    sourceSets { getByName("main") { jniLibs.srcDirs("../../../target/platform/android/jniLibs") } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
}
