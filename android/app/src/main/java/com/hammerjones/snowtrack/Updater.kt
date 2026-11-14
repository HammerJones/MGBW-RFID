package com.hammerjones.snowtrack

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import androidx.core.content.pm.PackageInfoCompat
import java.io.File

/** The Android side of in-app updates: version lookup and handing an APK to the system installer. */
object Updater {
    private const val PREFS = "updates"
    private const val LAST_CHECK = "last_check_s"

    fun currentBuild(context: Context): Int = try {
        PackageInfoCompat.getLongVersionCode(context.packageManager.getPackageInfo(context.packageName, 0)).toInt()
    } catch (e: Exception) {
        0
    }

    fun versionName(context: Context): String = try {
        context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "?"
    } catch (e: Exception) {
        "?"
    }

    /** Where a given build is downloaded to. Older downloads are cleared out. */
    fun apkFile(context: Context, build: Int): File {
        val dir = File(context.cacheDir, "updates")
        dir.mkdirs()
        val wanted = File(dir, "SnowTrack-$build.apk")
        dir.listFiles()?.filter { it != wanted && !it.name.startsWith("SnowTrack-$build.") }?.forEach { it.delete() }
        return wanted
    }

    /** Automatic checks run at most every six hours. */
    fun dueForCheck(context: Context): Boolean {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        return System.currentTimeMillis() / 1000 - prefs.getLong(LAST_CHECK, 0) > 6 * 3600
    }

    fun markChecked(context: Context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putLong(LAST_CHECK, System.currentTimeMillis() / 1000).apply()
    }

    /** Android asks once per app before it may install anything. */
    fun canInstall(context: Context): Boolean = context.packageManager.canRequestPackageInstalls()

    fun openInstallPermission(context: Context) {
        val intent = Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + context.packageName))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(intent)
        } catch (e: Exception) {
            // Some devices lack this screen; the installer will prompt instead.
        }
    }

    /** Opens the system installer, which replaces the app and offers to reopen it. */
    fun install(context: Context, apk: File): Boolean = try {
        val uri = FileProvider.getUriForFile(context, context.packageName + ".files", apk)
        val intent = Intent(Intent.ACTION_VIEW)
            .setDataAndType(uri, "application/vnd.android.package-archive")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
        true
    } catch (e: Exception) {
        false
    }
}
