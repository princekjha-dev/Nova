package dev.nova.core

/**
 * JNI Binding layer connecting Android Kotlin runtime with Nova Rust Core.
 * Handles encrypted transport, Noise XX handshake, and CRDT synchronization.
 */
object NovaCoreBinding {
    init {
        try {
            System.loadLibrary("nova_android_jni")
        } catch (e: UnsatisfiedLinkError) {
            // Emulated in test / mock environment
        }
    }

    external fun init(dataDir: String): Long
    external fun pollEvents(handle: Long): String
    external fun sendMessage(handle: Long, msgJson: String): String
    external fun generatePairingQr(handle: Long): String
    external fun confirmPairing(handle: Long, candidateJson: String): Boolean
    external fun sendClipboard(handle: Long, text: String, targetDeviceId: String): Boolean
}
