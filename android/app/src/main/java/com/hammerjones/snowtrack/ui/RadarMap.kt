package com.hammerjones.snowtrack.ui

import android.annotation.SuppressLint
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.maplibre.android.MapLibre
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style
import org.maplibre.android.style.layers.PropertyFactory
import org.maplibre.android.style.layers.RasterLayer
import org.maplibre.android.style.sources.RasterSource
import org.maplibre.android.style.sources.TileSet
import uniffi.snowtrack_core.SnowTrack

private const val RADAR_OPACITY = 0.75f

/** USGS topo base map with a dot on the resort. Radar layers are added on top in code. */
private fun styleJson(lat: Double, lon: Double): String = """
{
  "version": 8,
  "sources": {
    "base": {
      "type": "raster",
      "tiles": ["https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/{z}/{y}/{x}"],
      "tileSize": 256,
      "maxzoom": 16,
      "attribution": "USGS The National Map · Radar: NOAA/NWS MRMS"
    },
    "resort": {
      "type": "geojson",
      "data": {"type": "Feature", "geometry": {"type": "Point", "coordinates": [$lon, $lat]}, "properties": {}}
    }
  },
  "layers": [
    {"id": "base", "type": "raster", "source": "base", "paint": {"raster-saturation": -0.35}},
    {"id": "resort", "type": "circle", "source": "resort",
     "paint": {"circle-radius": 6, "circle-color": "#0B1420", "circle-stroke-color": "#FFFFFF", "circle-stroke-width": 2.5}}
  ]
}
""".trimIndent()

private fun frameLabel(time: String?): String {
    if (time == null) return "Latest radar"
    return try {
        DateTimeFormatter.ofPattern("EEE h:mm a", Locale.US).withZone(ZoneId.systemDefault()).format(Instant.parse(time))
    } catch (e: Exception) {
        "Radar"
    }
}

/**
 * Live NOAA radar centred on a resort. Shows the latest frame straight away,
 * then loads the recent frames so the user can play the last couple of hours.
 */
@SuppressLint("ClickableViewAccessibility")
@Composable
fun RadarMap(lat: Double, lon: Double, core: SnowTrack, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    var style by remember { mutableStateOf<Style?>(null) }
    var frames by remember { mutableStateOf<List<String>>(emptyList()) }
    var framesAdded by remember { mutableStateOf(false) }
    var index by remember { mutableIntStateOf(-1) } // -1 = the "latest" layer
    var playing by remember { mutableStateOf(false) }

    val mapView = remember {
        MapLibre.getInstance(context)
        MapView(context).apply {
            // Let the map pan and zoom even though it sits inside a scrolling page.
            setOnTouchListener { view, _ ->
                view.parent?.requestDisallowInterceptTouchEvent(true)
                false
            }
            getMapAsync { map ->
                map.uiSettings.isRotateGesturesEnabled = false
                map.uiSettings.isTiltGesturesEnabled = false
                map.setMaxZoomPreference(11.0)
                map.setMinZoomPreference(3.0)
                map.cameraPosition = CameraPosition.Builder().target(LatLng(lat, lon)).zoom(6.3).build()
                map.setStyle(Style.Builder().fromJson(styleJson(lat, lon))) { loaded ->
                    loaded.addSource(RasterSource("radar-latest", TileSet("2.1.0", core.radarTileTemplate(lat, lon, null)), 256))
                    loaded.addLayerBelow(
                        RasterLayer("radar-latest", "radar-latest").withProperties(
                            PropertyFactory.rasterOpacity(RADAR_OPACITY),
                            PropertyFactory.rasterFadeDuration(0f),
                        ),
                        "resort",
                    )
                    style = loaded
                }
            }
        }
    }

    // MapView needs to follow the screen's lifecycle.
    DisposableEffect(lifecycleOwner, mapView) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_START -> mapView.onStart()
                Lifecycle.Event.ON_RESUME -> mapView.onResume()
                Lifecycle.Event.ON_PAUSE -> mapView.onPause()
                Lifecycle.Event.ON_STOP -> mapView.onStop()
                else -> Unit
            }
        }
        mapView.onCreate(null)
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            style = null
            mapView.onPause()
            mapView.onStop()
            mapView.onDestroy()
        }
    }

    LaunchedEffect(lat, lon) {
        frames = withContext(Dispatchers.IO) {
            try {
                core.radarTimes(lat, lon)
            } catch (e: Exception) {
                emptyList()
            }
        }
    }

    // Add one hidden layer per frame the first time the user presses play.
    LaunchedEffect(style, frames, playing) {
        val s = style ?: return@LaunchedEffect
        if (!playing || framesAdded || frames.isEmpty()) return@LaunchedEffect
        frames.forEachIndexed { i, time ->
            s.addSource(RasterSource("radar-$i", TileSet("2.1.0", core.radarTileTemplate(lat, lon, time)), 256))
            s.addLayerBelow(
                RasterLayer("radar-$i", "radar-$i").withProperties(
                    PropertyFactory.rasterOpacity(0f),
                    PropertyFactory.rasterFadeDuration(0f),
                ),
                "resort",
            )
        }
        framesAdded = true
    }

    LaunchedEffect(playing, framesAdded) {
        if (!playing || !framesAdded) return@LaunchedEffect
        // Give the first pass time to download tiles, then loop, pausing on the newest frame.
        while (true) {
            index = if (index < 0 || index >= frames.lastIndex) 0 else index + 1
            delay(if (index == frames.lastIndex) 1600 else 650)
        }
    }

    LaunchedEffect(index, framesAdded, style) {
        val s = style ?: return@LaunchedEffect
        s.getLayer("radar-latest")?.setProperties(PropertyFactory.rasterOpacity(if (index < 0) RADAR_OPACITY else 0f))
        if (framesAdded) {
            frames.indices.forEach { i ->
                s.getLayer("radar-$i")?.setProperties(PropertyFactory.rasterOpacity(if (i == index) RADAR_OPACITY else 0f))
            }
        }
    }

    Box(modifier.clip(RoundedCornerShape(20.dp))) {
        AndroidView(factory = { mapView }, modifier = Modifier.fillMaxSize())
        Row(
            Modifier
                .align(Alignment.TopStart)
                .padding(8.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.92f))
                .padding(start = 12.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                frameLabel(frames.getOrNull(index)),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
            if (frames.size > 1) {
                TextButton(onClick = {
                    playing = !playing
                    if (!playing) index = -1
                }) { Text(if (playing) "Stop" else "Play loop") }
            } else {
                Box(Modifier.padding(vertical = 14.dp, horizontal = 4.dp))
            }
        }
    }
}
