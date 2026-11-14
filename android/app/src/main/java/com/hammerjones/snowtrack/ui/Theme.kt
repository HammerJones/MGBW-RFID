package com.hammerjones.snowtrack.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// "Alpine night": deep blue-black with ice blue for snow that has fallen
// and a warmer dawn tone for snow that is only forecast.
private val Night = darkColorScheme(
    primary = Color(0xFF8FD3FF),
    onPrimary = Color(0xFF00344F),
    primaryContainer = Color(0xFF14415F),
    onPrimaryContainer = Color(0xFFCDEBFF),
    secondary = Color(0xFFFFC98A),
    onSecondary = Color(0xFF452B00),
    tertiary = Color(0xFFB9C8FF),
    background = Color(0xFF0B1420),
    onBackground = Color(0xFFE3EAF2),
    surface = Color(0xFF0B1420),
    onSurface = Color(0xFFE3EAF2),
    surfaceVariant = Color(0xFF16222F),
    onSurfaceVariant = Color(0xFF9FB0C2),
    outline = Color(0xFF3A4A5C),
    outlineVariant = Color(0xFF223040),
    error = Color(0xFFFFB4A9),
    errorContainer = Color(0xFF5C1D16),
    onErrorContainer = Color(0xFFFFDAD4),
)

private val Day = lightColorScheme(
    primary = Color(0xFF0B6A9C),
    onPrimary = Color.White,
    primaryContainer = Color(0xFFCDEBFF),
    onPrimaryContainer = Color(0xFF001E30),
    secondary = Color(0xFFB45F00),
    onSecondary = Color.White,
    tertiary = Color(0xFF4557A8),
    background = Color(0xFFF3F7FB),
    onBackground = Color(0xFF111C27),
    surface = Color(0xFFF3F7FB),
    onSurface = Color(0xFF111C27),
    surfaceVariant = Color(0xFFFFFFFF),
    onSurfaceVariant = Color(0xFF52657A),
    outline = Color(0xFFB7C5D3),
    outlineVariant = Color(0xFFDCE5EE),
    error = Color(0xFFB3261E),
    errorContainer = Color(0xFFFFDAD4),
    onErrorContainer = Color(0xFF410002),
)

@Composable
fun SnowTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) Night else Day, content = content)
}
