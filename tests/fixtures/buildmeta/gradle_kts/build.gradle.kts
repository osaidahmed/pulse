plugins {
    kotlin("jvm") version "1.9.0"
}

dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.0")
    api("com.google.guava:guava:32.0.0-jre")
    testImplementation("junit:junit:4.13.2")
    implementation(group = "org.apache.commons", name = "commons-lang3", version = "3.12.0")
    implementation(kotlin("reflect"))
    implementation(libs.retrofit)
}
