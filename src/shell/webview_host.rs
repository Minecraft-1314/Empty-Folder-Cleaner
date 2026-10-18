use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Window, WindowBuilder};
use wry::http::{Request, Response, StatusCode};
use wry::{WebContext, WebView, WebViewBuilder};

use crate::bridge::contracts::{response, BridgeException};
use crate::bridge::router::{parse_request, CommandRouter};
use crate::bridge::runtime::{AppRuntime, EventSink};
use crate::core::app_log;
use crate::core::localization;
use crate::core::settings_store::SettingsStore;

const CUSTOM_PROTOCOL: &str = "app";
const ENTRY_PAGE: &str = "http://app.localhost/index.html";
const MAX_MESSAGE_CHARACTERS: usize = 4 * 1024 * 1024;
const WINDOW_WIDTH: f64 = 1280.0;
const WINDOW_HEIGHT: f64 = 820.0;
const MIN_WIDTH: f64 = 360.0;
const MIN_HEIGHT: f64 = 560.0;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

const BRIDGE_SHIM: &str = r#"(function () {
  var listeners = [];
  window.__hostPush = function (text) {
    var data = typeof text === "string" ? JSON.parse(text) : text;
    for (var index = 0; index < listeners.length; index += 1) {
      try { listeners[index]({ data: data }); } catch (error) { }
    }
  };
  var chromeRef = window.chrome || {};
  var nativeWebview = chromeRef.webview;
  var nativePost = nativeWebview && typeof nativeWebview.postMessage === "function"
    ? nativeWebview.postMessage.bind(nativeWebview)
    : null;
  window.chrome = chromeRef;
  window.chrome.webview = {
    postMessage: function (message) {
      if (!nativePost) { throw new Error("bridge_unavailable"); }
      nativePost(typeof message === "string" ? message : JSON.stringify(message));
    },
    addEventListener: function (type, handler) {
      if (type === "message" && typeof handler === "function") { listeners.push(handler); }
    },
    removeEventListener: function (type, handler) {
      var index = listeners.indexOf(handler);
      if (index >= 0) { listeners.splice(index, 1); }
    }
  };
})();"#;

pub enum UserEvent {
    Response(String),
    Failed(String),
}

pub fn run() -> i32 {
    app_log::bind();

    let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
    let event_loop = builder.build();
    let proxy = event_loop.create_proxy();

    let web_directory = match resolve_web_directory() {
        Ok(value) => value,
        Err(error) => {
            app_log::error(&error);
            show_fallback(&error);
            return 1;
        }
    };

    let icon_directory = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let window = match build_window(&event_loop, &icon_directory) {
        Ok(value) => value,
        Err(error) => {
            app_log::error(&error.to_string());
            show_fallback(&localization::resolve(
                &localization::detect(),
                localization::KEY_FALLBACK_START_FAILED,
            ));
            return 1;
        }
    };

    let sink: Arc<dyn EventSink> = Arc::new(ProxySink {
        proxy: proxy.clone(),
    });
    let runtime = Arc::new(AppRuntime::new(sink, SettingsStore::default()));
    let router = Arc::new(CommandRouter::new(runtime.clone()));

    let webview = match build_webview(&window, web_directory, router.clone(), proxy.clone()) {
        Ok(value) => value,
        Err(error) => {
            app_log::error(&error.to_string());
            show_fallback(&localization::resolve(
                &localization::detect(),
                localization::KEY_FALLBACK_START_FAILED,
            ));
            return 1;
        }
    };

    let shutdown_runtime = runtime.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                shutdown_runtime.shutdown(SHUTDOWN_TIMEOUT);
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Response(text)) => {
                let _ = webview.evaluate_script(&format!("window.__hostPush({text});"));
            }
            Event::UserEvent(UserEvent::Failed(message)) => {
                app_log::error(&message);
            }
            _ => {}
        }
    });
}

struct ProxySink {
    proxy: EventLoopProxy<UserEvent>,
}

impl EventSink for ProxySink {
    fn post(&self, payload: Value) {
        match serde_json::to_string(&payload) {
            Ok(text) => {
                let _ = self.proxy.send_event(UserEvent::Response(text));
            }
            Err(error) => {
                let _ = self.proxy.send_event(UserEvent::Failed(error.to_string()));
            }
        }
    }
}

fn build_window(
    event_loop: &tao::event_loop::EventLoop<UserEvent>,
    icon_directory: &Path,
) -> Result<Window, tao::error::OsError> {
    let mut builder = WindowBuilder::new()
        .with_title(localization::resolve(
            &localization::detect(),
            localization::KEY_TITLE,
        ))
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(MIN_WIDTH, MIN_HEIGHT));
    if let Some(icon) = crate::shell::icon::load_icon(icon_directory) {
        builder = builder.with_window_icon(Some(icon));
    }
    builder.build(event_loop)
}

fn resolve_user_data_directory() -> PathBuf {
    std::env::temp_dir()
        .join("EmptyFolderCleaner")
        .join("WebView2")
}

fn resolve_web_directory() -> Result<PathBuf, String> {
    let base = std::env::current_exe()
        .map_err(|error| error.to_string())?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "resource.missing".to_string())?;

    for candidate in [
        base.join("Ui").join("Web"),
        base.join("Assets").join("Ui").join("Web"),
    ] {
        if candidate.join("index.html").is_file() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "{} {}",
        localization::resolve(
            &localization::detect(),
            localization::KEY_FALLBACK_MISSING_FILES
        ),
        base.join("Ui").join("Web").to_string_lossy()
    ))
}

fn build_webview(
    window: &Window,
    web_directory: PathBuf,
    router: Arc<CommandRouter>,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<WebView, wry::Error> {
    let directory = web_directory.clone();
    let handler_router = router.clone();
    let handler_proxy = proxy.clone();
    let navigation_router = router.clone();

    let mut context = WebContext::new(Some(resolve_user_data_directory()));
    WebViewBuilder::with_web_context(&mut context)
        .with_custom_protocol(CUSTOM_PROTOCOL.to_string(), move |_id, request| {
            serve(&directory, request)
        })
        .with_initialization_script(BRIDGE_SHIM)
        .with_ipc_handler(move |request| {
            let reply = handle_message(&handler_router, request.body());
            match serde_json::to_string(&reply) {
                Ok(text) => {
                    let _ = handler_proxy.send_event(UserEvent::Response(text));
                }
                Err(error) => {
                    let _ = handler_proxy.send_event(UserEvent::Failed(error.to_string()));
                }
            }
        })
        .with_navigation_handler(move |url| {
            if is_internal_url(&url) {
                return true;
            }
            let _ = crate::bridge::platform::open_external(&url);
            let _ = navigation_router.runtime().current_language();
            false
        })
        .with_devtools(false)
        .with_url(ENTRY_PAGE)
        .build(window)
}

pub fn is_internal_url(url: &str) -> bool {
    let http = format!("http://{CUSTOM_PROTOCOL}.localhost/");
    let https = format!("https://{CUSTOM_PROTOCOL}.localhost/");
    let scheme = format!("{CUSTOM_PROTOCOL}://");
    url.starts_with(&http) || url.starts_with(&https) || url.starts_with(&scheme)
}

fn serve(directory: &Path, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let relative = request_path(request.uri());
    if relative.contains("..") {
        return forbidden();
    }

    let target = directory.join(&relative);
    match std::fs::read(&target) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", content_type(&relative))
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(bytes))
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

pub fn request_path(uri: &wry::http::Uri) -> String {
    let raw = uri.to_string();
    let authority = raw.find("://").map(|index| index + 3).unwrap_or(0);
    let after = &raw[authority..];
    let start = after.find('/').unwrap_or(after.len());
    let path = &after[start..];
    let path = match path.find(['?', '#']) {
        Some(index) => &path[..index],
        None => path,
    };
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        "index.html".to_string()
    } else {
        trimmed.to_string()
    }
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Borrowed(&b"not found"[..]))
        .expect("valid response")
}

fn forbidden() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Borrowed(&b"forbidden"[..]))
        .expect("valid response")
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "map" => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn handle_message(router: &CommandRouter, raw: &str) -> Value {
    let request = match parse_request(raw, MAX_MESSAGE_CHARACTERS) {
        Ok(value) => value,
        Err(exception) => {
            return failure(Value::Null, &exception);
        }
    };

    match router.execute(&request.action, &request.payload) {
        Ok(result) => response(&request.id, true, result, None),
        Err(exception) => failure(request.id.clone(), &exception),
    }
}

fn failure(id: Value, exception: &BridgeException) -> Value {
    let fields: Vec<&str> = exception
        .fields
        .iter()
        .map(|value| value.as_str())
        .collect();
    response(&id, false, Value::Null, Some((&exception.code, &fields)))
}

fn show_fallback(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    let title: Vec<u16> = localization::resolve(&localization::detect(), localization::KEY_TITLE)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
