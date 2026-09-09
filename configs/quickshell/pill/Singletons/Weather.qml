pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

/**
 * Live weather for the pill's hover glance, served by Open-Meteo with no API key.
 * Location resolves once and is cached so a restart never re-hits the network for
 * coordinates: by default the city, latitude and longitude come from a keyless IP
 * lookup (ip-api), but a non-empty `Flags.weatherCity` override geocodes that name
 * via Open-Meteo's geocoder instead. Once coordinates are known the forecast runs
 * immediately and then every 20 minutes, exposing the current conditions plus a
 * 24-hour hourly strip.
 *
 * Everything is async through `Process` + `curl`, mirroring how Sysmon and Devices
 * fetch, so startup never blocks on a slow or absent connection. Every JSON parse
 * is guarded: a partial body or network blip leaves the last good values in place
 * and `ready` simply stays false until the first clean fetch lands.
 *
 * Conditions render as on-brand kanji rather than icons — 晴 clear, 曇 cloud,
 * 雨 rain, 雪 snow, 霧 fog, 雷 thunder, 月 a clear night — keyed off the WMO weather
 * code via `glyphFor`, with `labelFor` giving the short english word.
 */
Singleton {
    id: root

    readonly property string cacheDir: (Quickshell.env("XDG_CACHE_HOME") || (Quickshell.env("HOME") + "/.cache")) + "/ricelin"

    property int tempNow: 0
    property int codeNow: 0
    property int humidity: 0
    property bool isDay: true
    property string city: ""
    property var hourly: []
    property var daily: []
    property bool ready: false

    /** The detail fields the weather surface shows; 0/"" until a fetch lands. */
    property int windNow: 0
    property int windDir: -1
    property real precipNow: 0
    property int pressureNow: 0
    property real uvNow: 0
    property string sunrise: ""
    property string sunset: ""
    property int feelsNow: 0

    /**
     * The active backend: "open-meteo" (keyless default) or "accuweather"
     * (needs Flags.weatherKey). The switch re-locates and re-fetches; an
     * AccuWeather pick without a key falls back to Open-Meteo and reports
     * through `backendNote` so the surface can say why.
     */
    readonly property string backend: {
        var want = Flags.weatherBackend || "open-meteo";
        if (want === "accuweather" && (!Flags.weatherKey || Flags.weatherKey.length === 0))
            return "open-meteo";
        return want;
    }
    readonly property string backendNote: {
        var want = Flags.weatherBackend || "open-meteo";
        if (want === "accuweather" && backend !== "accuweather")
            return "AccuWeather needs an API key — using Open-Meteo";
        return "";
    }

    property real lat: 0
    property real lon: 0
    property bool located: false

    /**
     * Maps a WMO weather code to its on-brand kanji. Clear skies show 月 at night
     * so the glance reads day-versus-night at a glance; every other condition is
     * the same glyph round the clock.
     */
    function glyphFor(code, day) {
        if (code === 0)
            return day ? "sun" : "moon";
        if (code <= 3)
            return "cloud";
        if (code === 45 || code === 48)
            return "cloud-fog";
        if (code >= 95)
            return "cloud-lightning";
        if ((code >= 71 && code <= 77) || code === 85 || code === 86)
            return "cloud-snow";
        if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82))
            return "cloud-rain";
        return "cloud";
    }

    /** Short english word for a WMO weather code, for labels and accessibility. */
    function labelFor(code) {
        if (code === 0)
            return "Clear";
        if (code <= 3)
            return "Cloudy";
        if (code === 45 || code === 48)
            return "Fog";
        if (code >= 95)
            return "Thunder";
        if ((code >= 71 && code <= 77) || code === 85 || code === 86)
            return "Snow";
        if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82))
            return "Rain";
        return "Cloudy";
    }

    /** Persist resolved coordinates so a restart skips the location round-trip. */
    function writeLoc() {
        locCache.setText(JSON.stringify({ city: root.city, lat: root.lat, lon: root.lon }));
    }

    function fetchWeather() {
        if (!located)
            return;
        if (backend === "accuweather") {
            if (_accuKey.length > 0 && !accuProc.running)
                accuProc.running = true;
            else if (!accuLocProc.running && !accuProc.running)
                accuLocProc.running = true;
            return;
        }
        if (wxProc.running)
            return;
        wxProc.running = true;
    }

    /**
     * AccuWeather's condition codes onto the WMO codes the rest of the shell
     * reads (glyphFor/labelFor). Only the families the surface distinguishes:
     * clear, cloudy, fog, rain, snow, thunder. Unknown codes read as cloudy.
     */
    function accuToWmo(c) {
        if (c === 1 || c === 2 || c === 30 || c === 33 || c === 34)
            return 0;                       // clear / sunny / mostly sunny
        if (c === 3 || c === 4 || c === 5 || c === 6 || c === 35 || c === 36)
            return 2;                       // partly/mostly cloudy
        if (c === 7 || c === 8)
            return 45;                      // cloudy / overcast
        if (c === 11)
            return 45;                      // fog
        if (c >= 12 && c <= 18)
            return 61;                      // showers / rain variants
        if (c >= 19 && c <= 29)
            return 71;                      // snow variants (19-29 incl. sleet)
        if (c === 32 || c === 37 || c === 38)
            return 95;                      // thunderstorms
        if (c === 39 || c === 40 || c === 41 || c === 42)
            return 61;                      // scattered/all-day rain
        if (c === 43 || c === 44)
            return 71;                      // scattered/all-day snow
        return 3;
    }

    /**
     * Loads cached coordinates synchronously (blockLoading) and fetches at once;
     * an absent or malformed cache falls through to a fresh location lookup.
     */
    Component.onCompleted: {
        try {
            var c = JSON.parse(locCache.text());
            if (c && typeof c.lat === "number" && typeof c.lon === "number") {
                root.city = c.city || "";
                root.lat = c.lat;
                root.lon = c.lon;
                root.located = true;
                root.fetchWeather();
                return;
            }
        } catch (e) {}
        root.locate();
    }

    FileView {
        id: locCache
        path: root.cacheDir + "/weather-loc.json"
        blockLoading: true
        printErrors: false
    }

    /** Resolve coordinates: geocode the manual city override, else fall back to IP. */
    function locate() {
        if (Flags.weatherCity && Flags.weatherCity.trim().length > 0)
            geoProc.running = true;
        else
            ipProc.running = true;
    }

    Connections {
        target: Flags
        function onWeatherCityChanged() { root.locate(); }
        function onWeatherBackendChanged() { root.locate(); }
        function onWeatherKeyChanged() { root.locate(); }
    }

    Process {
        id: ipProc
        command: ["curl", "-s", "--max-time", "8", "http://ip-api.com/json?fields=lat,lon,city"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    if (typeof d.lat === "number" && typeof d.lon === "number") {
                        root.city = d.city || "";
                        root.lat = d.lat;
                        root.lon = d.lon;
                        root.located = true;
                        root.writeLoc();
                        root.fetchWeather();
                    }
                } catch (e) {}
            }
        }
    }

    Process {
        id: geoProc
        command: ["curl", "-s", "--max-time", "8", "-G",
            "https://geocoding-api.open-meteo.com/v1/search",
            "--data-urlencode", "name=" + (Flags.weatherCity || ""),
            "--data-urlencode", "count=1"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    var r = d.results && d.results[0];
                    if (r && typeof r.latitude === "number" && typeof r.longitude === "number") {
                        root.city = r.name || "";
                        root.lat = r.latitude;
                        root.lon = r.longitude;
                        root.located = true;
                        root.writeLoc();
                        root.fetchWeather();
                    }
                } catch (e) {}
            }
        }
    }

    Process {
        id: wxProc
        command: ["curl", "-s", "--max-time", "10",
            "https://api.open-meteo.com/v1/forecast?latitude=" + root.lat
            + "&longitude=" + root.lon
            + "&current=temperature_2m,apparent_temperature,weather_code,is_day,relative_humidity_2m,wind_speed_10m,wind_direction_10m,precipitation,surface_pressure"
            + "&hourly=temperature_2m,weather_code&forecast_hours=24"
            + "&daily=weather_code,temperature_2m_max,temperature_2m_min,relative_humidity_2m_mean,uv_index_max,sunrise,sunset&forecast_days=5&timezone=auto"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    var cur = d.current;
                    if (!cur)
                        return;
                    var rows = [];
                    var h = d.hourly;
                    if (h && h.time && h.temperature_2m && h.weather_code) {
                        var n = Math.min(h.time.length, h.temperature_2m.length, h.weather_code.length);
                        for (var i = 0; i < n; i++) {
                            rows.push({
                                hour: h.time[i].slice(11, 13),
                                temp: Math.round(h.temperature_2m[i]),
                                code: h.weather_code[i]
                            });
                        }
                    }
                    var days = [];
                    var dd = d.daily;
                    if (dd && dd.time && dd.weather_code && dd.temperature_2m_max && dd.relative_humidity_2m_mean) {
                        var dn = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                        var m = Math.min(dd.time.length, dd.weather_code.length, dd.temperature_2m_max.length, dd.relative_humidity_2m_mean.length);
                        for (var j = 0; j < m; j++) {
                            days.push({
                                day: dn[new Date(dd.time[j]).getDay()],
                                code: dd.weather_code[j],
                                temp: Math.round(dd.temperature_2m_max[j]),
                                min: dd.temperature_2m_min ? Math.round(dd.temperature_2m_min[j]) : 0,
                                rh: Math.round(dd.relative_humidity_2m_mean[j]),
                                uv: dd.uv_index_max ? Math.round(dd.uv_index_max[j]) : 0
                            });
                        }
                    }
                    root.tempNow = Math.round(cur.temperature_2m);
                    root.codeNow = cur.weather_code;
                    root.humidity = Math.round(cur.relative_humidity_2m);
                    root.isDay = cur.is_day === 1;
                    root.feelsNow = Math.round(cur.apparent_temperature);
                    root.windNow = Math.round(cur.wind_speed_10m);
                    root.windDir = typeof cur.wind_direction_10m === "number" ? Math.round(cur.wind_direction_10m) : -1;
                    root.precipNow = typeof cur.precipitation === "number" ? cur.precipitation : 0;
                    root.pressureNow = Math.round(cur.surface_pressure);
                    root.uvNow = 0;
                    if (dd && dd.sunrise && dd.sunrise.length > 0) {
                        root.sunrise = dd.sunrise[0].slice(11, 16);
                        root.sunset = dd.sunset && dd.sunset.length > 0 ? dd.sunset[0].slice(11, 16) : "";
                    }
                    root.hourly = rows;
                    root.daily = days;
                    root.ready = true;
                } catch (e) {}
            }
        }
    }

    /**
     * AccuWeather: the location key lookup first, then current + 12h hourly +
     * 5-day daily. Two chained processes with the key cached in memory; the
     * city-text search matches Flags.weatherCity or the IP city, falling back
     * to the coordinates' nearest match (the geoposition endpoint, which needs
     * no city text). Daily carries temp min/max, day/night phrases.
     */
    property string _accuKey: ""

    Process {
        id: accuLocProc
        command: ["curl", "-s", "--max-time", "10",
            "https://dataservice.accuweather.com/locations/v1/cities/geoposition/search"
            + "?apikey=" + encodeURIComponent(Flags.weatherKey || "")
            + "&q=" + root.lat + "," + root.lon]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    if (d && d.Key) {
                        root._accuKey = d.Key;
                        if (d.LocalizedName && (!Flags.weatherCity || Flags.weatherCity.length === 0))
                            root.city = d.LocalizedName;
                        accuProc.running = true;
                    }
                } catch (e) {}
            }
        }
    }

    Process {
        id: accuProc
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    if (d && d.length > 0 && d[0].Temperature && d[0].WeatherIcon !== undefined) {
                        root.tempNow = Math.round(d[0].Temperature.Metric.Value);
                        root.codeNow = accuToWmo(d[0].WeatherIcon);
                        root.humidity = d[0].RelativeHumidity || 0;
                        root.isDay = !!d[0].IsDaylight;
                        root.feelsNow = Math.round(d[0].RealFeel ? d[0].RealFeel.Metric.Value : d[0].Temperature.Metric.Value);
                        root.windNow = Math.round(d[0].Wind ? d[0].Wind.Speed.Value : 0);
                        root.windDir = d[0].Wind ? d[0].Wind.Direction.Degrees : -1;
                        root.precipNow = d[0].Precip1hr ? d[0].Precip1hr.Metric.Value : 0;
                        root.pressureNow = d[0].Pressure ? Math.round(d[0].Pressure.Metric.Value) : 0;
                        root.ready = true;
                    }
                } catch (e) {}
            }
        }
        onRunningChanged: if (running) {
            // Built at fire time: the location key lands asynchronously.
            var base = "https://dataservice.accuweather.com/currentconditions/v1/"
                + root._accuKey
                + "?apikey=" + encodeURIComponent(Flags.weatherKey || "")
                + "&details=true";
            // Hourly and daily ride their own fetches — one endpoint each.
            accuHourlyProc.command = ["curl", "-s", "--max-time", "10",
                "https://dataservice.accuweather.com/forecasts/v1/hourly/12hour/" + root._accuKey
                + "?apikey=" + encodeURIComponent(Flags.weatherKey || "") + "&metric=true"];
            accuDailyProc.command = ["curl", "-s", "--max-time", "10",
                "https://dataservice.accuweather.com/forecasts/v1/daily/5day/" + root._accuKey
                + "?apikey=" + encodeURIComponent(Flags.weatherKey || "") + "&metric=true"];
            command = ["curl", "-s", "--max-time", "10", base];
            accuHourlyProc.running = true;
            accuDailyProc.running = true;
        }
    }

    Process {
        id: accuHourlyProc
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    if (d && d.length > 0) {
                        var rows = [];
                        for (var i = 0; i < d.length; i++) {
                            var dt = new Date(d[i].DateTime);
                            rows.push({
                                hour: ("0" + dt.getHours()).slice(-2),
                                temp: Math.round(d[i].Temperature.Value),
                                code: accuToWmo(d[i].WeatherIcon)
                            });
                        }
                        root.hourly = rows;
                    }
                } catch (e) {}
            }
        }
    }

    Process {
        id: accuDailyProc
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    var d = JSON.parse(this.text);
                    if (d && d.DailyForecasts) {
                        var dn = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                        var days = [];
                        for (var i = 0; i < d.DailyForecasts.length; i++) {
                            var f = d.DailyForecasts[i];
                            days.push({
                                day: dn[new Date(f.Date).getDay()],
                                code: accuToWmo(f.Day.Icon),
                                temp: Math.round(f.Temperature.Maximum.Value),
                                min: Math.round(f.Temperature.Minimum.Value),
                                rh: 0,
                                uv: 0
                            });
                        }
                        root.daily = days;
                        root.sunrise = f_Sunrise(d);
                        root.sunset = f_Sunset(d);
                    }
                } catch (e) {}
            }
        }
    }

    function f_Sunrise(d) {
        try { return d.DailyForecasts[0].Sun.Rise.slice(11, 16); } catch (e) { return ""; }
    }
    function f_Sunset(d) {
        try { return d.DailyForecasts[0].Sun.Set.slice(11, 16); } catch (e) { return ""; }
    }

    Timer {
        interval: 1200000
        running: true
        repeat: true
        onTriggered: root.fetchWeather()
    }
}
