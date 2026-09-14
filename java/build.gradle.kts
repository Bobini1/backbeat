plugins {
    `java-library`
}

group = "ac.backbeat"
version = providers.gradleProperty("backbeatVersion").orElse("0.1.0-SNAPSHOT").get()

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
    withSourcesJar()
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.13.0")
    testImplementation(platform("org.junit:junit-bom:5.10.2"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testImplementation("org.xerial:sqlite-jdbc:3.42.0.0")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

val nativeLibrary = providers.gradleProperty("nativeLibrary")
val nativePlatform = providers.gradleProperty("nativePlatform")

val nativeJar = tasks.register<Jar>("nativeJar") {
    onlyIf { nativeLibrary.isPresent && nativePlatform.isPresent }
    archiveClassifier.set(nativePlatform)
    from(nativeLibrary.map(::file)) {
        into(nativePlatform.map { "ac/backbeat/sdk/natives/$it" })
    }
}

tasks.test {
    useJUnitPlatform()
    testLogging {
        events("failed")
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
    if (nativeLibrary.isPresent && nativePlatform.isPresent) {
        dependsOn(nativeJar)
        classpath += files(nativeJar)
        systemProperty("backbeat.integration", "true")
    }
}
