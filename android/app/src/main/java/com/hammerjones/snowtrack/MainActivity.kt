package com.hammerjones.snowtrack

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.key
import com.hammerjones.snowtrack.ui.AddResortScreen
import com.hammerjones.snowtrack.ui.HomeScreen
import com.hammerjones.snowtrack.ui.ResortScreen
import com.hammerjones.snowtrack.ui.SnowTheme

class MainActivity : ComponentActivity() {
    private val vm: SnowViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) handleIntent(intent)
        setContent {
            SnowTheme {
                // Keep the background check alive across reinstalls and reboots.
                LaunchedEffect(vm.loaded) {
                    if (vm.loaded) Notifier.sync(applicationContext, vm.cards.any { it.notify })
                }
                BackHandler(enabled = vm.screen != Screen.Home) { vm.go(Screen.Home) }
                when (val screen = vm.screen) {
                    Screen.Home -> HomeScreen(vm)
                    Screen.Add -> AddResortScreen(vm)
                    // Keyed so switching resorts (from a notification) rebuilds the map.
                    is Screen.Detail -> key(screen.id) { ResortScreen(vm, screen.id) }
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    override fun onResume() {
        super.onResume()
        if (vm.loaded) vm.refreshOutdated()
    }

    private fun handleIntent(intent: Intent?) {
        intent?.getStringExtra(Notifier.EXTRA_RESORT_ID)?.let { vm.openFromNotification(it) }
    }
}
