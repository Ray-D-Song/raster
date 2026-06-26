plugins {
    id("com.android.library")
    id("maven-publish")
    signing
}

group = "io.github.ray-d-song"
version = rasterReleaseVersion()

android {
    namespace = "dev.raster.android.runtime"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    publishing {
        singleVariant("release")
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("../src/main/jniLibs")
        }
    }

    packaging {
        jniLibs {
            keepDebugSymbols += listOf("**/*.so")
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
}

publishing {
    publications {
        create<MavenPublication>("release") {
            groupId = "io.github.ray-d-song"
            artifactId = "raster-android"
            version = project.version.toString()

            pom {
                name.set("Raster Android Runtime")
                description.set("Android native runtime for Raster applications.")
                url.set("https://github.com/Ray-D-Song/raster")
                licenses {
                    license {
                        name.set("MIT")
                        url.set("https://opensource.org/license/mit")
                    }
                }
                developers {
                    developer {
                        id.set("ray-d-song")
                        name.set("Ray-D-Song")
                    }
                }
                scm {
                    connection.set("scm:git:https://github.com/Ray-D-Song/raster.git")
                    developerConnection.set("scm:git:ssh://git@github.com/Ray-D-Song/raster.git")
                    url.set("https://github.com/Ray-D-Song/raster")
                }
            }
        }
    }

    repositories {
        maven {
            name = "localStaging"
            url = rootProject.layout.buildDirectory.dir("maven-staging").get().asFile.toURI()
        }
    }
}

afterEvaluate {
    publishing.publications.named<MavenPublication>("release") {
        from(components["release"])
    }
}

val signingKey = providers.gradleProperty("signingInMemoryKey")
    .orElse(providers.environmentVariable("SIGNING_KEY"))
    .orNull
val signingPassword = providers.gradleProperty("signingInMemoryKeyPassword")
    .orElse(providers.environmentVariable("SIGNING_PASSWORD"))
    .orNull

if (!signingKey.isNullOrBlank()) {
    signing {
        useInMemoryPgpKeys(signingKey, signingPassword)
        sign(publishing.publications["release"])
    }
}

fun rasterReleaseVersion(): String {
    val config = rootProject.layout.projectDirectory.file("../../config.json").asFile.readText()
    return Regex(""""version"\s*:\s*"([^"]+)"""")
        .find(config)
        ?.groupValues
        ?.get(1)
        ?: error("config.json must contain version")
}
