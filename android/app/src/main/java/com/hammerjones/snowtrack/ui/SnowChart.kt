package com.hammerjones.snowtrack.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.hammerjones.snowtrack.Format
import java.time.LocalDate
import uniffi.snowtrack_core.DaySnow

/**
 * Snow per day as bars: solid for what fell, outlined for the local
 * forecast, faint for the long-range model, with a divider at today. Scrolls
 * sideways; it opens with the last couple of days and the coming week in view.
 * Days with no data leave a gap rather than a zero bar.
 */
@Composable
fun SnowChart(
    days: List<DaySnow>,
    metric: Boolean,
    barAreaHeight: Dp,
    showValues: Boolean,
    modifier: Modifier = Modifier,
    columnWidth: Dp = 40.dp,
) {
    if (days.isEmpty()) return
    val firstForecast = days.indexOfFirst { it.isForecast }
    val today = days.getOrNull(firstForecast)?.date?.let { runCatching { LocalDate.parse(it) }.getOrNull() }
        ?: LocalDate.now()
    // Scale to the biggest day, but never let a dusting fill the chart.
    val floorCm = if (metric) 10.0 else 4 * 2.54
    val maxCm = maxOf(days.maxOfOrNull { it.snowCm ?: 0.0 } ?: 0.0, floorCm)
    val observed = MaterialTheme.colorScheme.primary
    val forecast = MaterialTheme.colorScheme.secondary
    val longRange = MaterialTheme.colorScheme.tertiary
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val summary = days.joinToString(", ") {
        val kind = if (it.longRange) " long range" else if (it.isForecast) " forecast" else ""
        "${Format.dayLabel(it.date, today)} ${Format.chartDate(it.date)} ${Format.snow(it.snowCm, metric)}$kind"
    }

    val scroll = rememberScrollState()
    val density = LocalDensity.current
    // Open two days before today so recent snow and the week ahead both show.
    LaunchedEffect(days.size, firstForecast) {
        val startColumn = (firstForecast - 2).coerceAtLeast(0)
        scroll.scrollTo(with(density) { (columnWidth * startColumn).roundToPx() })
    }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .horizontalScroll(scroll)
            .semantics { contentDescription = "Snow by day, scrolls sideways: $summary" },
        verticalAlignment = Alignment.Bottom,
    ) {
        days.forEachIndexed { index, day ->
            if (day.isForecast && index > 0 && !days[index - 1].isForecast) {
                // Divider between what happened and what is expected.
                Box(
                    Modifier
                        .padding(bottom = 32.dp)
                        .width(1.dp)
                        .height(barAreaHeight + if (showValues) 16.dp else 0.dp)
                        .background(MaterialTheme.colorScheme.outline)
                )
            }
            val cm = day.snowCm
            val fraction = if (cm == null) 0f else (cm / maxCm).toFloat().coerceIn(0f, 1f)
            val color = if (day.longRange) longRange else if (day.isForecast) forecast else observed
            val shape = RoundedCornerShape(topStart = 4.dp, topEnd = 4.dp)
            Column(Modifier.width(columnWidth), horizontalAlignment = Alignment.CenterHorizontally) {
                if (showValues) {
                    Text(
                        text = if (cm == null) "" else Format.snow(cm, metric).removeSuffix(" cm"),
                        fontSize = 10.sp,
                        lineHeight = 14.sp,
                        maxLines = 1,
                        softWrap = false,
                        color = if (cm != null && cm > 0.0) MaterialTheme.colorScheme.onSurface else muted,
                        modifier = Modifier.height(16.dp),
                    )
                }
                Box(Modifier.fillMaxWidth().height(barAreaHeight), contentAlignment = Alignment.BottomCenter) {
                    when {
                        cm == null -> Unit
                        fraction < 0.02f ->
                            // A zero day still gets a baseline tick so it reads as "none", not "unknown".
                            Box(Modifier.fillMaxWidth(0.66f).height(2.dp).background(muted.copy(alpha = 0.5f)))
                        day.isForecast ->
                            Box(
                                Modifier
                                    .fillMaxWidth(0.66f)
                                    .fillMaxHeight(fraction)
                                    .clip(shape)
                                    .background(color.copy(alpha = if (day.longRange) 0.14f else 0.28f))
                                    .border(if (day.longRange) 1.dp else 1.5.dp, color.copy(alpha = if (day.longRange) 0.7f else 1f), shape)
                            )
                        else ->
                            Box(Modifier.fillMaxWidth(0.66f).fillMaxHeight(fraction).clip(shape).background(color))
                    }
                }
                val isToday = runCatching { LocalDate.parse(day.date) == today }.getOrDefault(false)
                val labelColor = if (isToday) MaterialTheme.colorScheme.onSurface else muted
                Text(
                    text = if (isToday) "Now" else Format.dayLabel(day.date, today).take(2),
                    fontSize = 10.sp,
                    lineHeight = 14.sp,
                    maxLines = 1,
                    softWrap = false,
                    textAlign = TextAlign.Center,
                    fontWeight = if (isToday) FontWeight.Bold else FontWeight.Normal,
                    color = labelColor,
                    modifier = Modifier.height(16.dp),
                )
                Text(
                    text = Format.chartDate(day.date),
                    fontSize = 10.sp,
                    lineHeight = 14.sp,
                    maxLines = 1,
                    softWrap = false,
                    textAlign = TextAlign.Center,
                    fontWeight = if (isToday) FontWeight.Bold else FontWeight.Normal,
                    color = labelColor,
                    modifier = Modifier.height(16.dp),
                )
            }
        }
    }
}

/** Key for the chart colours. */
@Composable
fun ChartLegend(modifier: Modifier = Modifier) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Box(Modifier.width(10.dp).height(10.dp).clip(RoundedCornerShape(2.dp)).background(MaterialTheme.colorScheme.primary))
        Text("Fell", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Box(Modifier.width(6.dp))
        Box(
            Modifier
                .width(10.dp)
                .height(10.dp)
                .clip(RoundedCornerShape(2.dp))
                .background(MaterialTheme.colorScheme.secondary.copy(alpha = 0.28f))
                .border(1.5.dp, MaterialTheme.colorScheme.secondary, RoundedCornerShape(2.dp))
        )
        Text("Forecast", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Box(Modifier.width(6.dp))
        Box(
            Modifier
                .width(10.dp)
                .height(10.dp)
                .clip(RoundedCornerShape(2.dp))
                .background(MaterialTheme.colorScheme.tertiary.copy(alpha = 0.14f))
                .border(1.dp, MaterialTheme.colorScheme.tertiary.copy(alpha = 0.7f), RoundedCornerShape(2.dp))
        )
        Text("Long range (rough)", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
