
use serde_json::{json, Value};

pub fn build(user_agent: &str, language: &str) -> Value {
    let now = chrono::Local::now();
    let unix = now.timestamp();
    let stamp = now.format("%-m/%-d/%Y, %-I:%M:%S %p").to_string();
    let offset_minutes = -(now.offset().local_minus_utc() / 60);

    json!({
        "deviceInfo": {
            "cpuCores": 8,
            "cpuLoad": "Skipped",
            "deviceMemoryGB": 8,
            "platform": "Win32",
            "maxTouchPoints": 0,
            "msMaxTouchPoints": "Not Supported",
            "gpu": {
                "vendor": "Google Inc. (NVIDIA)",
                "renderer": "ANGLE (NVIDIA, NVIDIA GeForce GTX 1650 (0x00001F82) Direct3D11 vs_5_0 ps_5_0, D3D11)"
            },
            "battery": {
                "charging": true,
                "level": 1.0,
                "chargingTime": null,
                "dischargingTime": null
            },
            "stylusDetection": "No",
            "touchSupport": "No"
        },
        "browserInfo": {
            "userAgent": user_agent,
            "timezone": now.format("%Z").to_string(),
            "timezoneOffset": offset_minutes,
            "localeDateTime": stamp,
            "localUnixTime": unix,
            "calendar": "gregory",
            "day": "numeric",
            "locale": "en-US",
            "month": "numeric",
            "numberingSystem": "latn",
            "year": "numeric",
            "appName": "Netscape",
            "appVersion": user_agent.replace("Mozilla/", ""),
            "vendor": "Google Inc.",
            "language": language.split(',').next().unwrap_or("en-US"),
            "languages": language
                .split(',')
                .map(|part| part.split(';').next().unwrap_or(part).trim().to_string())
                .collect::<Vec<_>>(),
            "cookieEnabled": true,
            "onlineStatus": "Online",
            "javaEnabled": false,
            "doNotTrack": "Not Supported",
            "referrerHeader": "https://www.google.com/",
            "httpsConnection": "Yes",
            "historyLength": 2,
            "mimeTypes": 2,
            "plugins": 5,
            "webdriver": false,
            "pageVisibility": "visible",
            "isBot": "No",
            "featuresSupported": {
                "geolocation": "Yes",
                "serviceWorker": "Yes",
                "localStorage": "Yes",
                "sessionStorage": "Yes",
                "indexedDB": "Yes",
                "notifications": "Yes",
                "notificationsFirebase": "default",
                "clipboard": "Yes",
                "pushAPI": "Yes",
                "webRTC": "Yes",
                "gamepadAPI": "Yes",
                "speechSynthesis": "Yes",
                "webGL": "Yes",
                "vibrationAPI": "Yes",
                "deviceMotion": "Yes",
                "deviceOrientation": "Yes",
                "wakeLock": "Yes",
                "serial": "Yes",
                "usb": "Yes",
                "networkInformation": "Yes",
                "screenCapture": "Yes",
                "fullscreenAPI": "Yes",
                "pictureInPicture": "Yes"
            }
        },
        "screenInfo": {
            "width": 1920,
            "height": 1080,
            "colorDepth": 24,
            "pixelDepth": 24,
            "devicePixelRatio": 1.0,
            "orientation": "landscape-primary",
            "screenOrientationAngle": 0,
            "availableWidth": 1920,
            "availableHeight": 1040,
            "screenLeft": 0,
            "screenTop": 0,
            "outerWidth": 1920,
            "outerHeight": 1080,
            "innerWidth": 1920,
            "innerHeight": 947
        },
        "otherData": {
            "mouseAvailable": "Yes",
            "keyboardAvailable": "Yes",
            "bluetoothSupport": "Yes",
            "usbSupport": "Yes",
            "gamepadSupport": "Yes",
            "incognitoMode": "No"
        },
        "storageInfo": {
            "localStorage": 6,
            "sessionStorage": 0,
            "indexedDB": "Available",
            "cacheStorage": "Available",
            "storageEstimate": {
                "quota": 120000000000u64,
                "usage": 14320,
                "usageDetails": { "indexedDB": 14320 }
            }
        }
    })
}
