package com.hammerjones.snowtrack

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.PeriodicWorkRequest
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit
import uniffi.snowtrack_core.SnowNotice

/** Posts snow notifications and keeps the background check scheduled. */
object Notifier {
    const val EXTRA_RESORT_ID = "resort_id"
    private const val CHANNEL = "snow"
    private const val PERIODIC = "snow-check"
    private const val ONCE = "snow-check-now"

    fun canPost(context: Context): Boolean {
        val granted = Build.VERSION.SDK_INT < 33 ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
        return granted && NotificationManagerCompat.from(context).areNotificationsEnabled()
    }

    private fun ensureChannel(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        if (manager.getNotificationChannel(CHANNEL) == null) {
            val channel = NotificationChannel(CHANNEL, "Snow alerts", NotificationManager.IMPORTANCE_DEFAULT)
            channel.description = "Big snowfalls forecast and winter weather alerts for resorts with the bell on"
            manager.createNotificationChannel(channel)
        }
    }

    fun post(context: Context, notice: SnowNotice) {
        if (!canPost(context)) return
        ensureChannel(context)
        val open = Intent(context, MainActivity::class.java)
            .putExtra(EXTRA_RESORT_ID, notice.resortId)
            .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        val pending = PendingIntent.getActivity(
            context,
            notice.resortId.hashCode(),
            open,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_snow)
            .setContentTitle(notice.title)
            .setContentText(notice.body)
            .setStyle(NotificationCompat.BigTextStyle().bigText(notice.body))
            .setContentIntent(pending)
            .setAutoCancel(true)
            .build()
        try {
            // One slot per resort and kind of news, so a later update replaces the earlier one.
            NotificationManagerCompat.from(context).notify((notice.resortId + notice.title.take(12)).hashCode(), notification)
        } catch (e: SecurityException) {
            // Permission was revoked between the check and the post.
        }
    }

    /** Run the background check every few hours while any bell is on; stop it otherwise. */
    fun sync(context: Context, anyBellOn: Boolean, checkNow: Boolean = false) {
        val work = WorkManager.getInstance(context)
        if (!anyBellOn) {
            work.cancelUniqueWork(PERIODIC)
            return
        }
        val online = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
        val periodic = PeriodicWorkRequest.Builder(SnowWorker::class.java, 3, TimeUnit.HOURS)
            .setConstraints(online)
            .build()
        work.enqueueUniquePeriodicWork(PERIODIC, ExistingPeriodicWorkPolicy.KEEP, periodic)
        if (checkNow) {
            val once = OneTimeWorkRequest.Builder(SnowWorker::class.java).setConstraints(online).build()
            work.enqueueUniqueWork(ONCE, ExistingWorkPolicy.REPLACE, once)
        }
    }
}

class SnowWorker(context: Context, params: WorkerParameters) : Worker(context, params) {
    override fun doWork(): Result {
        val notices = try {
            Core.get(applicationContext).checkNotifications()
        } catch (e: Exception) {
            return Result.retry()
        }
        notices.forEach { Notifier.post(applicationContext, it) }
        return Result.success()
    }
}
