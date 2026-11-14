package com.hammerjones.snowtrack.ui

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Notifications
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material.icons.outlined.Notifications
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
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
import com.hammerjones.snowtrack.Notifier
import com.hammerjones.snowtrack.Screen
import com.hammerjones.snowtrack.SnowViewModel
import kotlin.math.roundToInt
import uniffi.snowtrack_core.Card
import uniffi.snowtrack_core.Settings

/** Shows view-model messages in a snackbar. */
@Composable
fun MessageHost(vm: SnowViewModel, host: SnackbarHostState) {
    val message = vm.message
    LaunchedEffect(message) {
        if (message != null) {
            vm.message = null
            host.showSnackbar(message)
        }
    }
}

/**
 * Returns a function that turns a resort's bell on or off, asking for the
 * notification permission first when Android requires it.
 */
@Composable
fun rememberBellToggle(vm: SnowViewModel): (String, Boolean) -> Unit {
    val context = LocalContext.current
    var pending by rememberSaveable { mutableStateOf<String?>(null) }
    val launcher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        val id = pending
        pending = null
        if (id != null) {
            if (granted) {
                vm.setNotify(id, true)
            } else {
                vm.message = "Notifications are turned off for SnowTrack. Allow them in Android settings to use the bell."
            }
        }
    }
    return { id, enable ->
        when {
            !enable -> vm.setNotify(id, false)
            Notifier.canPost(context) -> vm.setNotify(id, true)
            Build.VERSION.SDK_INT >= 33 -> {
                pending = id
                launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
            else -> vm.message = "Notifications are turned off for SnowTrack. Allow them in Android settings to use the bell."
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(vm: SnowViewModel) {
    val snackbar = remember { SnackbarHostState() }
    MessageHost(vm, snackbar)
    val toggleBell = rememberBellToggle(vm)
    var showSettings by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("SnowTrack", fontWeight = FontWeight.SemiBold) },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
                actions = {
                    IconButton(onClick = { vm.refreshAll() }, enabled = vm.cards.isNotEmpty()) {
                        Icon(Icons.Filled.Refresh, contentDescription = "Refresh all resorts")
                    }
                    IconButton(onClick = { showSettings = true }) {
                        Icon(Icons.Filled.Settings, contentDescription = "Settings")
                    }
                },
            )
        },
        floatingActionButton = {
            if (vm.cards.isNotEmpty()) {
                ExtendedFloatingActionButton(
                    onClick = { vm.go(Screen.Add) },
                    icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                    text = { Text("Add resort") },
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            if (vm.refreshing.isNotEmpty()) {
                LinearProgressIndicator(Modifier.fillMaxWidth())
            } else {
                Spacer(Modifier.height(4.dp))
            }
            vm.update?.let { info ->
                UpdateBanner(info.version, vm.downloadingUpdate, onUpdate = { vm.installUpdate() })
            }
            when {
                !vm.loaded -> Unit
                vm.cards.isEmpty() -> EmptyState(onAdd = { vm.go(Screen.Add) })
                else -> LazyColumn(
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 96.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    items(vm.cards, key = { it.resort.id }) { card ->
                        ResortCard(
                            card = card,
                            metric = vm.settings.metric,
                            refreshing = card.resort.id in vm.refreshing,
                            onOpen = { vm.go(Screen.Detail(card.resort.id)) },
                            onBell = { toggleBell(card.resort.id, !card.notify) },
                        )
                    }
                }
            }
        }
    }

    if (showSettings) {
        SettingsDialog(
            current = vm.settings,
            versionName = vm.versionName,
            checking = vm.checkingUpdate,
            onCheckUpdate = { vm.checkForUpdate(manual = true) },
            onDismiss = { showSettings = false },
            onSave = {
                vm.updateSettings(it)
                showSettings = false
            },
        )
    }
}

@Composable
private fun UpdateBanner(version: String, downloading: Boolean, onUpdate: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.primaryContainer,
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 8.dp),
    ) {
        Row(Modifier.padding(start = 16.dp, end = 8.dp, top = 6.dp, bottom = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("Update available", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.onPrimaryContainer)
                Text(
                    if (downloading) "Downloading $version…" else version,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            TextButton(onClick = onUpdate, enabled = !downloading) { Text("Update") }
        }
    }
}

@Composable
private fun EmptyState(onAdd: () -> Unit) {
    Column(
        Modifier.fillMaxSize().padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("No resorts yet", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(8.dp))
        Text(
            "Add the mountains you ski to see what fell, what's coming and the live radar.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(24.dp))
        Button(onClick = onAdd) {
            Icon(Icons.Filled.Add, contentDescription = null, modifier = Modifier.size(18.dp))
            Spacer(Modifier.size(8.dp))
            Text("Add a resort")
        }
    }
}

@Composable
fun BellButton(on: Boolean, name: String, onClick: () -> Unit) {
    IconButton(onClick = onClick) {
        if (on) {
            Icon(Icons.Filled.Notifications, contentDescription = "Turn off snow notifications for $name", tint = MaterialTheme.colorScheme.primary)
        } else {
            Icon(Icons.Outlined.Notifications, contentDescription = "Turn on snow notifications for $name", tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
fun Stat(label: String, value: String, modifier: Modifier = Modifier, emphasised: Boolean = false) {
    Column(modifier) {
        Text(
            value,
            style = if (emphasised) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            color = if (emphasised) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.onSurface,
            maxLines = 1,
        )
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
    }
}

@Composable
fun ResortCard(card: Card, metric: Boolean, refreshing: Boolean, onOpen: () -> Unit, onBell: () -> Unit) {
    val snap = card.snapshot
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(20.dp),
        modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(20.dp)).clickable(onClickLabel = "Open ${card.resort.name}", onClick = onOpen),
    ) {
        Column(Modifier.padding(start = 16.dp, end = 4.dp, top = 4.dp, bottom = 14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f).padding(top = 10.dp)) {
                    Text(card.resort.name, style = MaterialTheme.typography.titleLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    val base = Format.elevation(card.resort.baseElevM, metric)
                    val top = Format.elevation(card.resort.summitElevM, metric)
                    val where = listOfNotNull(
                        card.resort.state.ifBlank { null },
                        if (base != null && top != null) "${base.substringBefore(' ')}–$top" else null,
                    ).joinToString("  ·  ")
                    if (where.isNotEmpty()) {
                        Text(where, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                BellButton(card.notify, card.resort.name, onBell)
            }

            Column(Modifier.padding(end = 12.dp)) {
                if (snap == null) {
                    Spacer(Modifier.height(12.dp))
                    Text(
                        if (refreshing) "Loading snow data…" else "No data yet. Tap refresh when you're online.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    Spacer(Modifier.height(10.dp))
                    Row(verticalAlignment = Alignment.Bottom) {
                        Stat("next 3 days", Format.snow(snap.next72hCm, metric), Modifier.weight(1.3f), emphasised = true)
                        Stat("yesterday", Format.snow(snap.last24hCm, metric), Modifier.weight(1f))
                        Stat("next 7 days", Format.snow(snap.next7dCm, metric), Modifier.weight(1f))
                        Stat("on ground", Format.depth(snap.depthCm, metric), Modifier.weight(1f))
                    }
                    Spacer(Modifier.height(12.dp))
                    SnowChart(snap.days, metric, barAreaHeight = 56.dp, showValues = false, columnWidth = 36.dp)
                    snap.alerts.firstOrNull()?.let { alert ->
                        Spacer(Modifier.height(10.dp))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Filled.Warning, contentDescription = null, tint = MaterialTheme.colorScheme.error, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.size(6.dp))
                            Text(
                                alert.event + if (snap.alerts.size > 1) "  +${snap.alerts.size - 1} more" else "",
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.error,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                    val age = Format.age(System.currentTimeMillis() / 1000, snap.fetchedAtEpochS)
                    Text(
                        when {
                            refreshing -> "Updating…"
                            snap.problems.isNotEmpty() -> "Updated $age  ·  some data unavailable"
                            else -> "Updated $age"
                        },
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsDialog(
    current: Settings,
    versionName: String,
    checking: Boolean,
    onCheckUpdate: () -> Unit,
    onDismiss: () -> Unit,
    onSave: (Settings) -> Unit,
) {
    var metric by remember { mutableStateOf(current.metric) }
    // The slider works in whole inches (2–24) or 5 cm steps (5–60).
    var inches by remember { mutableFloatStateOf((current.notifyThresholdCm / 2.54).toFloat().coerceIn(2f, 24f)) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Settings") },
        text = {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Metric units", style = MaterialTheme.typography.bodyLarge)
                        Text("Centimetres, metres and °C", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = metric, onCheckedChange = { metric = it })
                }
                Spacer(Modifier.height(20.dp))
                Text("Notify me when this much is forecast", style = MaterialTheme.typography.bodyLarge)
                Text(
                    "${Format.threshold(inches.roundToInt() * 2.54, metric)} or more in the next 3 days, for resorts with the bell on",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Slider(value = inches, onValueChange = { inches = it }, valueRange = 2f..24f, steps = 21)
                Spacer(Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Version $versionName", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                    TextButton(onClick = onCheckUpdate, enabled = !checking) { Text(if (checking) "Checking…" else "Check for updates") }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { onSave(Settings(metric = metric, notifyThresholdCm = inches.roundToInt() * 2.54)) }) { Text("Save") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
