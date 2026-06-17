pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "RasterAndroid"
includeBuild("../../../packages/raster-android") {
    dependencySubstitution {
        substitute(module("io.github.ray-d-song:raster-android")).using(project(":raster-android"))
    }
}
include(":app")
