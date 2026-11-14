plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// CI passes -PversionCode=<run number> so each build installs over the last one.
val buildNumber = (project.findProperty("versionCode") as String?)?.toIntOrNull() ?: 1

android {
    namespace = "com.hammerjones.snowtrack"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.hammerjones.snowtrack"
        minSdk = 26
        targetSdk = 34
        versionCode = buildNumber
        versionName = "0.1.$buildNumber"
        // -Pabi=x86_64 builds for the emulator (used by the CI screenshot run).
        ndk { abiFilters += (project.findProperty("abi") as String?) ?: "arm64-v8a" }
    }

    signingConfigs {
        create("release") {
            storeFile = file(System.getenv("SNOWTRACK_KEYSTORE") ?: "snowtrack.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("release")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    buildFeatures { compose = true }
    composeOptions { kotlinCompilerExtensionVersion = "1.5.14" }

    // The Rust library (libsnowtrack_core.so) is copied into src/main/jniLibs by
    // scripts/build-android.sh, which also generates the Kotlin bindings.
    packaging { jniLibs { useLegacyPackaging = true } }

    lint {
        checkReleaseBuilds = false
        abortOnError = false
    }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.06.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.activity:activity-compose:1.9.0")
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
    implementation("androidx.work:work-runtime:2.9.0")
    // Rust bindings (UniFFI) talk to the native library through JNA.
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    // Map for the radar page.
    implementation("org.maplibre.gl:android-sdk:11.3.0")

    testImplementation("junit:junit:4.13.2")
}
