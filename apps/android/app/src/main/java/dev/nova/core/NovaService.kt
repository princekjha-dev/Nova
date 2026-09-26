package dev.nova.core

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.IBinder
import androidx.core.app.NotificationCompat
import dev.nova.MainActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class NovaService : Service() {
    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var novaHandle: Long = 0

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildNotification())
        initCore()
        startEventLoop()
    }

    private fun initCore() {
        novaHandle = try {
            NovaCoreBinding.init(filesDir.absolutePath)
        } catch (e: Exception) {
            0L
        }
    }

    private fun startEventLoop() {
        serviceScope.launch {
            while (isActive) {
                if (novaHandle != 0L) {
                    val eventsJson = NovaCoreBinding.pollEvents(novaHandle)
                    if (eventsJson.isNotEmpty() && eventsJson != "[]") {
                        // Broadcast or process incoming Nova messages
                    }
                }
                delay(100)
            }
        }
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Nova Background Service",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Maintains encrypted local connection to paired Linux PC"
        }
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification {
        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Nova is Connected")
            .setContentText("Syncing with Prince's Linux PC over LAN")
            .setSmallIcon(dev.nova.R.drawable.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        serviceScope.cancel()
        super.onDestroy()
    }

    companion object {
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "nova_connected_service"
    }
}
