package com.hammerjones.snowtrack

import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FormatTest {
    @Test
    fun snowImperial() {
        assertEquals("—", Format.snow(null, false))
        assertEquals("—", Format.snow(Double.NaN, false))
        assertEquals("0\"", Format.snow(0.0, false))
        assertEquals("<0.1\"", Format.snow(0.1, false))
        assertEquals("0.1\"", Format.snow(0.254, false))
        assertEquals("2.5\"", Format.snow(6.35, false))
        assertEquals("6\"", Format.snow(15.24, false))
        assertEquals("9.9\"", Format.snow(25.1, false))
        assertEquals("10\"", Format.snow(25.4, false))
        assertEquals("14\"", Format.snow(35.56, false))
    }

    @Test
    fun snowMetric() {
        assertEquals("0 cm", Format.snow(0.0, true))
        assertEquals("<0.1 cm", Format.snow(0.04, true))
        assertEquals("4.8 cm", Format.snow(4.8, true))
        assertEquals("36 cm", Format.snow(35.56, true))
    }

    @Test
    fun depthAndTemp() {
        assertEquals("21\"", Format.depth(53.34, false))
        assertEquals("53 cm", Format.depth(53.34, true))
        assertEquals("—", Format.depth(null, true))
        assertEquals("32°", Format.temp(0.0, false))
        assertEquals("-4°", Format.temp(-20.0, false))
        assertEquals("-20°", Format.temp(-20.0, true))
        assertEquals("—", Format.hiLo(null, null, false))
        assertEquals("34° / —", Format.hiLo(1.0, null, false))
    }

    @Test
    fun elevationAndDistance() {
        assertEquals("8,120 ft", Format.elevation(2474.976, false))
        assertEquals("2,475 m", Format.elevation(2474.976, true))
        assertNull(Format.elevation(null, false))
        assertEquals("3 mi", Format.distance(4.2, false))
        assertEquals("1 km", Format.distance(0.2, true))
    }

    @Test
    fun age() {
        assertEquals("", Format.age(1000, 0))
        assertEquals("just now", Format.age(1000, 950))
        assertEquals("12 min ago", Format.age(10_000, 10_000 - 12 * 60))
        assertEquals("3 h ago", Format.age(100_000, 100_000 - 3 * 3600 - 5))
        assertEquals("2 d ago", Format.age(1_000_000, 1_000_000 - 2 * 86400 - 5))
    }

    @Test
    fun dayLabels() {
        val today = LocalDate.of(2026, 11, 14)
        assertEquals("Today", Format.dayLabel("2026-11-14", today))
        assertEquals("Sun", Format.dayLabel("2026-11-15", today))
        assertEquals("Fri 13", Format.dayAndDate("2026-11-13", today))
        assertEquals("garbage", Format.dayLabel("garbage", today))
    }
}
