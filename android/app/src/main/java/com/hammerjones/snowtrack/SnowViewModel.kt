package com.hammerjones.snowtrack

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.snowtrack_core.Card
import uniffi.snowtrack_core.Resort
import uniffi.snowtrack_core.Settings
import uniffi.snowtrack_core.SnowTrack

sealed class Screen {
    data object Home : Screen()
    data object Add : Screen()
    data class Detail(val id: String) : Screen()
}

class SnowViewModel(app: Application) : AndroidViewModel(app) {
    val core: SnowTrack = Core.get(app)

    var screen by mutableStateOf<Screen>(Screen.Home)
        private set
    var cards by mutableStateOf<List<Card>>(emptyList())
        private set
    var loaded by mutableStateOf(false)
        private set
    var settings by mutableStateOf(Settings(metric = false, notifyThresholdCm = 15.24))
        private set
    /** Ids currently being refreshed. */
    var refreshing by mutableStateOf<Set<String>>(emptySet())
        private set
    /** One-off message for the snackbar. */
    var message by mutableStateOf<String?>(null)

    init {
        viewModelScope.launch {
            settings = withContext(Dispatchers.IO) { core.settings() }
            reload()
            loaded = true
            refreshOutdated()
        }
    }

    fun go(to: Screen) {
        screen = to
    }

    fun openFromNotification(id: String) {
        screen = Screen.Detail(id)
    }

    private suspend fun reload() {
        cards = withContext(Dispatchers.IO) { core.cards() }
    }

    private fun nowS() = System.currentTimeMillis() / 1000

    /** Refresh anything never loaded or more than half an hour old. */
    fun refreshOutdated() {
        val cutoff = nowS() - 30 * 60
        cards.filter { (it.snapshot?.fetchedAtEpochS ?: 0L) < cutoff }.forEach { refresh(it.resort.id, quiet = true) }
    }

    fun refreshAll() {
        cards.forEach { refresh(it.resort.id) }
    }

    fun refresh(id: String, quiet: Boolean = false) {
        if (id in refreshing) return
        refreshing = refreshing + id
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                try {
                    Result.success(core.refresh(id))
                } catch (e: Exception) {
                    Result.failure(e)
                }
            }
            refreshing = refreshing - id
            result.onSuccess { snap ->
                if (snap.stale && !quiet) message = "Can't reach the weather services. Showing saved data."
            }.onFailure { e ->
                if (!quiet) message = e.message ?: "Couldn't load snow data"
            }
            reload()
        }
    }

    fun search(query: String, onResult: (List<Resort>) -> Unit) {
        viewModelScope.launch {
            onResult(withContext(Dispatchers.IO) { core.searchCatalog(query) })
        }
    }

    fun add(resort: Resort) {
        viewModelScope.launch {
            val error = withContext(Dispatchers.IO) {
                try {
                    core.addResort(resort)
                    null
                } catch (e: Exception) {
                    e.message ?: "Couldn't add ${resort.name}"
                }
            }
            if (error != null) {
                message = error
            } else {
                reload()
                screen = Screen.Home
                refresh(resort.id)
            }
        }
    }

    fun addCustom(name: String, lat: Double, lon: Double) {
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                try {
                    Result.success(core.addCustom(name, lat, lon))
                } catch (e: Exception) {
                    Result.failure(e)
                }
            }
            result.onSuccess { resort ->
                reload()
                screen = Screen.Home
                refresh(resort.id)
            }.onFailure { e ->
                message = e.message ?: "Couldn't add that location"
            }
        }
    }

    fun remove(id: String) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { core.removeResort(id) }
            reload()
            screen = Screen.Home
            Notifier.sync(getApplication(), cards.any { it.notify })
        }
    }

    fun setNotify(id: String, enabled: Boolean) {
        viewModelScope.launch {
            withContext(Dispatchers.IO) { core.setNotify(id, enabled) }
            reload()
            Notifier.sync(getApplication(), cards.any { it.notify }, checkNow = enabled)
            if (enabled) {
                val name = cards.firstOrNull { it.resort.id == id }?.resort?.name ?: "this resort"
                message = "You'll be notified when ${Format.threshold(settings.notifyThresholdCm, settings.metric)} or more is forecast for $name"
            }
        }
    }

    fun updateSettings(new: Settings) {
        viewModelScope.launch {
            settings = withContext(Dispatchers.IO) {
                core.setSettings(new)
                core.settings()
            }
        }
    }
}
