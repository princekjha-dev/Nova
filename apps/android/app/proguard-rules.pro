# Nova Android Proguard Rules

# Preserve JNI Core Bindings
-keep class dev.nova.core.NovaCoreBinding { *; }
-keepclassmembers class dev.nova.core.NovaCoreBinding {
    native <methods>;
}

# Preserve Compose Runtime
-keep class androidx.compose.** { *; }
