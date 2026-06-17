plugins {
    id("com.android.application")
}

android {
    namespace = "dev.raster.android"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.raster.android"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        manifestPlaceholders["nativeLibraryName"] = "raster"
    }

    buildTypes {
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    sourceSets {
        getByName("main") {
            assets.srcDirs("src/main/assets")
        }
    }

}

dependencies {
    implementation("io.github.ray-d-song:raster-android:0.1.0-alpha.12")
}
