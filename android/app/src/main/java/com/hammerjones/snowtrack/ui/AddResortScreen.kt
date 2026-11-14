package com.hammerjones.snowtrack.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.hammerjones.snowtrack.Format
import com.hammerjones.snowtrack.Screen
import com.hammerjones.snowtrack.SnowViewModel
import uniffi.snowtrack_core.Resort

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AddResortScreen(vm: SnowViewModel) {
    val snackbar = remember { SnackbarHostState() }
    MessageHost(vm, snackbar)
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<Resort>>(emptyList()) }
    var searched by remember { mutableStateOf(false) }
    var showCustom by remember { mutableStateOf(false) }

    LaunchedEffect(query, vm.cards.size) {
        vm.search(query) {
            results = it
            searched = true
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Add a resort") },
                navigationIcon = {
                    IconButton(onClick = { vm.go(Screen.Home) }) { Icon(Icons.Filled.ArrowBack, contentDescription = "Back") }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background),
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize().imePadding()) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Resort or state") },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            )
            Spacer(Modifier.height(8.dp))
            LazyColumn(contentPadding = PaddingValues(bottom = 24.dp)) {
                items(results, key = { it.id }) { resort ->
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .clickable(onClickLabel = "Add ${resort.name}") { vm.add(resort) }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(resort.name, style = MaterialTheme.typography.bodyLarge)
                            val top = Format.elevation(resort.summitElevM, vm.settings.metric)
                            Text(
                                listOfNotNull(resort.state.ifBlank { null }, top?.let { "summit $it" }).joinToString("  ·  "),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Icon(Icons.Filled.Add, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
                }
                item {
                    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        if (searched && results.isEmpty()) {
                            Text(
                                if (query.isBlank()) "You've added every resort in the list." else "No resort matches \"${query.trim()}\".",
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        TextButton(onClick = { showCustom = true }) { Text("Add a place by coordinates") }
                    }
                }
            }
        }
    }

    if (showCustom) {
        CustomPlaceDialog(
            initialName = query.trim(),
            onDismiss = { showCustom = false },
            onAdd = { name, lat, lon ->
                showCustom = false
                vm.addCustom(name, lat, lon)
            },
        )
    }
}

@Composable
private fun CustomPlaceDialog(initialName: String, onDismiss: () -> Unit, onAdd: (String, Double, Double) -> Unit) {
    var name by remember { mutableStateOf(initialName) }
    var lat by remember { mutableStateOf("") }
    var lon by remember { mutableStateOf("") }
    val latValue = lat.trim().toDoubleOrNull()
    val lonValue = lon.trim().toDoubleOrNull()
    val numbers = KeyboardOptions(keyboardType = KeyboardType.Text)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Add a place") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    "Any spot in the United States, such as a backcountry zone. Find coordinates by long-pressing in a maps app.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                OutlinedTextField(value = name, onValueChange = { name = it }, label = { Text("Name") }, singleLine = true)
                OutlinedTextField(
                    value = lat,
                    onValueChange = { lat = it },
                    label = { Text("Latitude, e.g. 39.798") },
                    singleLine = true,
                    isError = lat.isNotBlank() && latValue == null,
                    keyboardOptions = numbers,
                )
                OutlinedTextField(
                    value = lon,
                    onValueChange = { lon = it },
                    label = { Text("Longitude, e.g. -105.777") },
                    singleLine = true,
                    isError = lon.isNotBlank() && lonValue == null,
                    keyboardOptions = numbers,
                )
            }
        },
        confirmButton = {
            TextButton(
                enabled = name.isNotBlank() && latValue != null && lonValue != null,
                onClick = { if (latValue != null && lonValue != null) onAdd(name, latValue, lonValue) },
            ) { Text("Add") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
