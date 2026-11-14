package com.hammerjones.snowtrack

import android.content.Context
import java.io.File
import uniffi.snowtrack_core.SnowTrack

/** The one shared handle to the Rust core. */
object Core {
    @Volatile
    private var instance: SnowTrack? = null

    fun get(context: Context): SnowTrack =
        instance ?: synchronized(this) {
            instance ?: SnowTrack(File(context.applicationContext.filesDir, "snowtrack").absolutePath)
                .also { instance = it }
        }
}
