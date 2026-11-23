plugins {
    id("com.android.application") version "8.5.0"
}

android {
    namespace = "com.rtrust.mobile"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.rtrust.mobile"
        minSdk = 26
        targetSdk = 34
        versionCode = 4
        versionName = "0.3.4"

        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }

    sourceSets {
        getByName("main") {
            manifest.srcFile("AndroidManifest.xml")
            jniLibs.srcDirs("jniLibs")
            java.srcDirs("java")
            res.srcDirs("res")
        }
    }

    buildTypes {
        debug {
            isMinifyEnabled = false
        }
        release {
            isMinifyEnabled = false
        }
    }
}
