package com.hammerjones.snowtrack.ui

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.hammerjones.snowtrack.Format
import com.hammerjones.snowtrack.Screen
import com.hammerjones.snowtrack.SnowViewModel
import java.time.LocalDate
import uniffi.snowtrack_core.Alert
import uniffi.snowtrack_core.DaySnow
import uniffi.snowtrack_core.NewsItem
import uniffi.snowtrack_core.Snapshot

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ResortScreen(vm: SnowViewModel, id: String) {
    val snackbar = remember { SnackbarHostState() }
    MessageHost(vm, snackbar)
    val toggleBell = rememberBellToggle(vm)
    val card = vm.cards.firstOrNull { it.resort.id == id }
    var confirmRemove by remember { mutableStateOf(false) }

    // The resort was removed (or a notification pointed at one that no longer exists).
    LaunchedEffect(card == null, vm.loaded) {
        if (vm.loaded && card == null) vm.go(Screen.Home)
    }
    if (card == null) return

    val metric = vm.settings.metric
    val snap = card.snapshot
    val refreshing = id in vm.refreshing

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(card.resort.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { vm.go(Screen.Home) }) { Icon(Icons.Filled.ArrowBack, contentDescription = "Back") }
                },
                actions = {
                    BellButton(card.notify, card.resort.name) { toggleBell(id, !card.notify) }
                    IconButton(onClick = { vm.refresh(id) }, enabled = !refreshing) {
                        Icon(Icons.Filled.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(onClick = { confirmRemove = true }) {
                        Icon(Icons.Filled.Delete, contentDescription = "Remove ${card.resort.name}")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            if (refreshing) LinearProgressIndicator(Modifier.fillMaxWidth()) else Spacer(Modifier.height(4.dp))
            Column(
                Modifier.verticalScroll(rememberScrollState()).padding(horizontal = 16.dp).padding(bottom = 32.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                RadarMap(card.resort.lat, card.resort.lon, vm.core, mapHeight = 300.dp)

                if (snap == null) {
                    Text(
                        if (refreshing) "Loading snow data…" else "No snow data yet. Tap refresh when you're online.",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    StatusLine(snap)
                    snap.alerts.forEach { AlertCard(it) }
                    Totals(snap, metric)
                    Section("Snow by day") {
                        SnowChart(snap.days, metric, barAreaHeight = 120.dp, showValues = true, columnWidth = 44.dp)
                        Spacer(Modifier.height(8.dp))
                        ChartLegend()
                    }
                    Section("Day by day") { DayTable(snap, metric) }
                    Attribution(snap, metric)
                    snap.outlook?.let { Section("Forecaster's outlook") { Outlook(it) } }
                    Section("In the news") { News(snap.news, metric) }
                }
            }
        }
    }

    if (confirmRemove) {
        AlertDialog(
            onDismissRequest = { confirmRemove = false },
            title = { Text("Remove ${card.resort.name}?") },
            text = { Text("It will come off your list. You can add it again any time.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmRemove = false
                    vm.remove(id)
                }) { Text("Remove") }
            },
            dismissButton = { TextButton(onClick = { confirmRemove = false }) { Text("Keep") } },
        )
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    Surface(color = MaterialTheme.colorScheme.surfaceVariant, shape = RoundedCornerShape(20.dp), modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Spacer(Modifier.height(12.dp))
            content()
        }
    }
}

@Composable
private fun StatusLine(snap: Snapshot) {
    val age = Format.age(System.currentTimeMillis() / 1000, snap.fetchedAtEpochS)
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(
            if (snap.stale) "Offline. Showing data from $age" else "Updated $age",
            style = MaterialTheme.typography.labelMedium,
            color = if (snap.stale) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
        )
        snap.problems.forEach {
            Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
    }
}

@Composable
private fun AlertCard(alert: Alert) {
    Surface(color = MaterialTheme.colorScheme.errorContainer, shape = RoundedCornerShape(16.dp), modifier = Modifier.fillMaxWidth()) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.Top) {
            Icon(Icons.Filled.Warning, contentDescription = null, tint = MaterialTheme.colorScheme.onErrorContainer, modifier = Modifier.size(20.dp))
            Spacer(Modifier.size(10.dp))
            Column {
                Text(alert.event, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onErrorContainer, fontWeight = FontWeight.SemiBold)
                if (alert.headline != alert.event) {
                    Text(alert.headline, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onErrorContainer)
                }
            }
        }
    }
}

@Composable
private fun Totals(snap: Snapshot, metric: Boolean) {
    Surface(color = MaterialTheme.colorScheme.surfaceVariant, shape = RoundedCornerShape(20.dp), modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
            Row {
                Stat("next 24 hours", Format.snow(snap.next24hCm, metric), Modifier.weight(1f), emphasised = true)
                Stat("next 3 days", Format.snow(snap.next72hCm, metric), Modifier.weight(1f), emphasised = true)
                Stat("next 7 days", Format.snow(snap.next7dCm, metric), Modifier.weight(1f), emphasised = true)
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            Row {
                Stat("yesterday", Format.snow(snap.last24hCm, metric), Modifier.weight(1f))
                Stat("last 3 days", Format.snow(snap.last72hCm, metric), Modifier.weight(1f))
                Stat("last 7 days", Format.snow(snap.last7dCm, metric), Modifier.weight(1f))
                Stat("on ground", Format.depth(snap.depthCm, metric), Modifier.weight(1f))
            }
        }
    }
}

@Composable
private fun DayTable(snap: Snapshot, metric: Boolean) {
    val today = snap.days.firstOrNull { it.isForecast }?.date?.let { runCatching { LocalDate.parse(it) }.getOrNull() } ?: LocalDate.now()
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    Column {
        Row(Modifier.padding(bottom = 6.dp)) {
            Text("Day", Modifier.weight(1.4f), style = MaterialTheme.typography.labelMedium, color = muted)
            Text("Snow", Modifier.weight(1f), style = MaterialTheme.typography.labelMedium, color = muted, textAlign = TextAlign.End)
            Text("On ground", Modifier.weight(1.2f), style = MaterialTheme.typography.labelMedium, color = muted, textAlign = TextAlign.End)
            Text("High / low", Modifier.weight(1.4f), style = MaterialTheme.typography.labelMedium, color = muted, textAlign = TextAlign.End)
        }
        snap.days.forEachIndexed { i, day ->
            if (day.isForecast && (i == 0 || !snap.days[i - 1].isForecast)) {
                Text(
                    "FORECAST",
                    Modifier.fillMaxWidth().padding(top = 10.dp, bottom = 4.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.secondary,
                    fontWeight = FontWeight.Bold,
                )
            }
            if (day.longRange && (i == 0 || !snap.days[i - 1].longRange)) {
                Text(
                    "LONG RANGE  ·  GFS model, a rough guide",
                    Modifier.fillMaxWidth().padding(top = 10.dp, bottom = 4.dp),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.tertiary,
                    fontWeight = FontWeight.Bold,
                )
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            DayRow(day, today, metric)
        }
    }
}

@Composable
private fun DayRow(day: DaySnow, today: LocalDate, metric: Boolean) {
    val snowing = (day.snowCm ?: 0.0) > 0.0
    Row(Modifier.padding(vertical = 9.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(Format.dayAndDate(day.date, today), Modifier.weight(1.4f), style = MaterialTheme.typography.bodyMedium)
        Text(
            Format.snow(day.snowCm, metric),
            Modifier.weight(1f),
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = if (snowing) FontWeight.Bold else FontWeight.Normal,
            color = when {
                !snowing -> MaterialTheme.colorScheme.onSurfaceVariant
                day.longRange -> MaterialTheme.colorScheme.tertiary
                day.isForecast -> MaterialTheme.colorScheme.secondary
                else -> MaterialTheme.colorScheme.primary
            },
            textAlign = TextAlign.End,
        )
        Text(Format.depth(day.depthCm, metric), Modifier.weight(1.2f), style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.End)
        Text(Format.hiLo(day.tempHiC, day.tempLoC, metric), Modifier.weight(1.4f), style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.End)
    }
}

@Composable
private fun Attribution(snap: Snapshot, metric: Boolean) {
    val station = snap.station
    val history = if (station != null) {
        val elev = Format.elevation(station.elevM, metric)?.let { " at $it" } ?: ""
        "Past snow is the daily rise in snow depth at the ${station.name} SNOTEL station$elev, ${Format.distance(station.distanceKm, metric)} away. It can read low when snow settles or blows."
    } else {
        "No SNOTEL snow station within range of this resort, so past snowfall isn't available."
    }
    Text(
        "$history Forecast: National Weather Service. Long-range days: NOAA's GFS model via Open-Meteo, useful for spotting a storm pattern but not for amounts.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 4.dp),
    )
}

@Composable
private fun Outlook(text: String) {
    var expanded by remember { mutableStateOf(false) }
    Text(
        text,
        style = MaterialTheme.typography.bodyMedium,
        maxLines = if (expanded) Int.MAX_VALUE else 6,
        overflow = TextOverflow.Ellipsis,
    )
    TextButton(onClick = { expanded = !expanded }) { Text(if (expanded) "Show less" else "Read more") }
    Text("From the local National Weather Service office", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

@Composable
private fun News(items: List<NewsItem>, metric: Boolean) {
    val context = LocalContext.current
    if (items.isEmpty()) {
        Text("No recent stories mention this resort.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        return
    }
    val now = System.currentTimeMillis() / 1000
    Column {
        items.forEachIndexed { i, item ->
            if (i > 0) HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            Column(
                Modifier
                    .fillMaxWidth()
                    .clickable(onClickLabel = "Open story") {
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(item.url)))
                        } catch (e: Exception) {
                            // No browser installed: nothing sensible to do.
                        }
                    }
                    .padding(vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                Text(item.title, style = MaterialTheme.typography.bodyMedium)
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        listOf(item.source, Format.age(now, item.publishedEpochS)).filter { it.isNotBlank() }.joinToString("  ·  "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    item.reportedSnowCm?.let {
                        Text(
                            "${Format.snow(it, metric)} reported",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.onPrimaryContainer,
                            modifier = Modifier
                                .clip(RoundedCornerShape(6.dp))
                                .background(MaterialTheme.colorScheme.primaryContainer)
                                .padding(horizontal = 6.dp, vertical = 2.dp),
                        )
                    }
                }
            }
        }
    }
}
