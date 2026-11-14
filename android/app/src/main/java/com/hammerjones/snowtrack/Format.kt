package com.hammerjones.snowtrack

import java.time.LocalDate
import java.time.format.TextStyle
import java.util.Locale
import kotlin.math.roundToInt
import kotlin.math.roundToLong

/** Turns the core's metric numbers into what the user reads. No Android imports: unit-tested on the JVM. */
object Format {
    const val NONE = "—"
    private const val CM_PER_IN = 2.54

    private fun oneDecimal(v: Double): String {
        val tenths = (v * 10).roundToLong()
        return if (tenths % 10 == 0L) (tenths / 10).toString() else "${tenths / 10}.${tenths % 10}"
    }

    /** A snowfall amount: `—`, `0"`, `<0.1"`, `2.5"`, `14"`. */
    fun snow(cm: Double?, metric: Boolean): String {
        if (cm == null || cm.isNaN()) return NONE
        val unit = if (metric) " cm" else "\""
        val v = if (metric) cm else cm / CM_PER_IN
        return when {
            v <= 0.0 -> "0$unit"
            v < 0.05 -> "<0.1$unit"
            v < 9.95 -> oneDecimal(v) + unit
            else -> "${v.roundToInt()}$unit"
        }
    }

    /** Snow on the ground: whole units only. */
    fun depth(cm: Double?, metric: Boolean): String {
        if (cm == null || cm.isNaN()) return NONE
        return if (metric) "${cm.roundToInt()} cm" else "${(cm / CM_PER_IN).roundToInt()}\""
    }

    fun temp(c: Double?, metric: Boolean): String {
        if (c == null || c.isNaN()) return NONE
        return if (metric) "${c.roundToInt()}°" else "${(c * 9 / 5 + 32).roundToInt()}°"
    }

    fun hiLo(hi: Double?, lo: Double?, metric: Boolean): String =
        if (hi == null && lo == null) NONE else "${temp(hi, metric)} / ${temp(lo, metric)}"

    fun elevation(m: Double?, metric: Boolean): String? {
        if (m == null || m.isNaN()) return null
        val v = if (metric) m.roundToInt() else (m / 0.3048).roundToInt()
        return String.format(Locale.US, "%,d", v) + if (metric) " m" else " ft"
    }

    fun distance(km: Double, metric: Boolean): String =
        if (metric) "${km.roundToInt().coerceAtLeast(1)} km" else "${(km / 1.609344).roundToInt().coerceAtLeast(1)} mi"

    /** How long ago something happened. */
    fun age(nowS: Long, thenS: Long): String {
        val s = nowS - thenS
        return when {
            thenS <= 0L -> ""
            s < 90 -> "just now"
            s < 3600 -> "${s / 60} min ago"
            s < 48 * 3600 -> "${s / 3600} h ago"
            else -> "${s / 86400} d ago"
        }
    }

    /** `Sat`, or `Today` for today's date. Falls back to the raw text. */
    fun dayLabel(date: String, today: LocalDate): String = try {
        val d = LocalDate.parse(date)
        if (d == today) "Today" else d.dayOfWeek.getDisplayName(TextStyle.SHORT, Locale.US)
    } catch (e: Exception) {
        date
    }

    /** `Sat 14`. */
    fun dayAndDate(date: String, today: LocalDate): String = try {
        val d = LocalDate.parse(date)
        if (d == today) "Today" else "${d.dayOfWeek.getDisplayName(TextStyle.SHORT, Locale.US)} ${d.dayOfMonth}"
    } catch (e: Exception) {
        date
    }

    /** Slider steps for the notification threshold, shown in the user's units. */
    fun threshold(cm: Double, metric: Boolean): String =
        if (metric) "${cm.roundToInt()} cm" else "${(cm / CM_PER_IN).roundToInt()}\""
}
