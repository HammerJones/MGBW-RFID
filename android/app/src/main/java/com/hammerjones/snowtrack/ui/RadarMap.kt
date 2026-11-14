package com.hammerjones.snowtrack.ui

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.graphics.ColorMatrix
import android.graphics.ColorMatrixColorFilter
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
import org.osmdroid.config.Configuration
import org.osmdroid.tileprovider.MapTileProviderBasic
import org.osmdroid.tileprovider.tilesource.OnlineTileSourceBase
import org.osmdroid.util.GeoPoint
import org.osmdroid.util.MapTileIndex
import org.osmdroid.views.CustomZoomButtonsController
import org.osmdroid.views.MapView
import org.osmdroid.views.overlay.Marker
import org.osmdroid.views.overlay.TilesOverlay
import uniffi.snowtrack_core.SnowTrack

private const val RADAR_OPACITY = 0.75f
private const val WORLD = 20037508.342789244

/** USGS The National Map topo tiles: public domain, no key. */
private object UsgsTopo : OnlineTileSourceBase(
    "USGSTopo", 0, 16, 256, "",
    arrayOf("https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/"),
) {
    override fun getTileURLString(pMapTileIndex: Long): String =
        baseUrl + MapTileIndex.getZoom(pMapTileIndex) + "/" + MapTileIndex.getY(pMapTileIndex) + "/" + MapTileIndex.getX(pMapTileIndex)
}

/**
 * NOAA radar served as WMS: each tile asks for its own Web-Mercator bounding
 * box. [name] doubles as the cache key, so every frame gets its own.
 */
private class RadarSource(name: String, private val template: String) :
    OnlineTileSourceBase(name, 0, 11, 256, ".png", arrayOf(template)) {
    override fun getTileURLString(pMapTileIndex: Long): String {
        val z = MapTileIndex.getZoom(pMapTileIndex)
        val size = 2 * WORLD / (1 shl z)
        val minX = -WORLD + MapTileIndex.getX(pMapTileIndex) * size
        val maxY = WORLD - MapTileIndex.getY(pMapTileIndex) * size
        val bbox = String.format(Locale.US, "%.3f,%.3f,%.3f,%.3f", minX, maxY - size, minX + size, maxY)
        return template.replace("{bbox-epsg-3857}", bbox)
    }
}

private fun radarOverlay(context: Context, map: MapView, name: String, template: String): TilesOverlay {
    val provider = MapTileProviderBasic(context, RadarSource(name, template))
    provider.tileRequestCompleteHandlers.add(map.tileRequestCompleteHandler)
    return TilesOverlay(provider, context).apply {
        loadingBackgroundColor = Color.TRANSPARENT
        loadingLineColor = Color.TRANSPARENT
        setColorFilter(ColorMatrixColorFilter(ColorMatrix().apply { setScale(1f, 1f, 1f, RADAR_OPACITY) }))
    }
}

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

    var frames by remember { mutableStateOf<List<String>>(emptyList()) }
    var frameOverlays by remember { mutableStateOf<List<TilesOverlay>>(emptyList()) }
    var index by remember { mutableIntStateOf(-1) } // -1 = the "latest" layer
    var playing by remember { mutableStateOf(false) }

    val mapView = remember {
        Configuration.getInstance().apply {
            load(context, context.getSharedPreferences("osmdroid", Context.MODE_PRIVATE))
            userAgentValue = "SnowTrack/0.1 (github.com/HammerJones)"
            tileFileSystemCacheMaxBytes = 60L * 1024 * 1024
            tileFileSystemCacheTrimBytes = 40L * 1024 * 1024
        }
        MapView(context).apply {
            setTileSource(UsgsTopo)
            setMultiTouchControls(true)
            isTilesScaledToDpi = true
            isHorizontalMapRepetitionEnabled = false
            isVerticalMapRepetitionEnabled = false
            zoomController.setVisibility(CustomZoomButtonsController.Visibility.NEVER)
            minZoomDouble = 3.0
            maxZoomDouble = 11.0
            controller.setZoom(6.5)
            controller.setCenter(GeoPoint(lat, lon))
            // Let the map pan and zoom even though it sits inside a scrolling page.
            setOnTouchListener { view, _ ->
                view.parent?.requestDisallowInterceptTouchEvent(true)
                false
            }
        }
    }
    // The newest frame changes every couple of minutes; a time bucket in the
    // name keeps an old cached copy from being shown as "latest".
    val latest = remember {
        val bucket = System.currentTimeMillis() / (5 * 60 * 1000)
        radarOverlay(context, mapView, "radar-latest-$bucket", core.radarTileTemplate(lat, lon, null)).also { overlay ->
            mapView.overlays.add(overlay)
            mapView.overlays.add(
                Marker(mapView).apply {
                    position = GeoPoint(lat, lon)
                    setAnchor(Marker.ANCHOR_CENTER, Marker.ANCHOR_BOTTOM)
                    setInfoWindow(null)
                }
            )
        }
    }

    DisposableEffect(lifecycleOwner, mapView) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_RESUME -> mapView.onResume()
                Lifecycle.Event.ON_PAUSE -> mapView.onPause()
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            mapView.onPause()
            mapView.onDetach()
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
    LaunchedEffect(frames, playing) {
        if (!playing || frameOverlays.isNotEmpty() || frames.isEmpty()) return@LaunchedEffect
        val region = if (lat > 50) "ak" else if (lat < 23) "hi" else "us"
        val added = frames.map { time ->
            val name = "radar-$region-" + time.filter { it.isLetterOrDigit() }
            radarOverlay(context, mapView, name, core.radarTileTemplate(lat, lon, time)).apply { isEnabled = false }
        }
        // Keep the marker on top.
        val at = (mapView.overlays.size - 1).coerceAtLeast(0)
        mapView.overlays.addAll(at, added)
        frameOverlays = added
    }

    LaunchedEffect(playing, frameOverlays) {
        if (!playing || frameOverlays.isEmpty()) return@LaunchedEffect
        // Loop through the frames, pausing on the newest.
        while (true) {
            index = if (index < 0 || index >= frames.lastIndex) 0 else index + 1
            delay(if (index == frames.lastIndex) 1600 else 700)
        }
    }

    LaunchedEffect(index, frameOverlays) {
        latest.isEnabled = index < 0
        frameOverlays.forEachIndexed { i, overlay -> overlay.isEnabled = i == index }
        mapView.invalidate()
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
        Text(
            "Map: USGS  ·  Radar: NOAA",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier
                .align(Alignment.BottomEnd)
                .padding(6.dp)
                .clip(RoundedCornerShape(6.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.85f))
                .padding(horizontal = 6.dp, vertical = 2.dp),
        )
    }
}
