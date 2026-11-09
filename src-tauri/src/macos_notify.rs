//! macOS 原生通知（UNUserNotificationCenter 直连）。
//!
//! tauri-plugin-notification 桌面端没有点击回调，点通知无法唤起面板；
//! 这里自建 delegate 接管点击事件：用户点横幅 → 主线程唤起面板。
//! `init` 必须在应用完成启动前于主线程调用（delegate 装晚了点击回调不生效）。
//! 移植自 gitCIStatus（Glint），机制一致。

use std::sync::{Mutex, OnceLock};

use block2::{DynBlock, RcBlock};
use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{msg_send, ClassType};
use objc2_foundation::{NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationDefaultActionIdentifier,
    UNNotification, UNNotificationPresentationOptions, UNNotificationRequest,
    UNNotificationResponse, UNNotificationSound, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use tauri::AppHandle;

/// delegate 回调要用的 AppHandle（delegate 无状态，走全局）
static PANEL_APP: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

/// Send/Sync 包装：delegate 对象无 ivars，回调由系统队列串行派发，仅用于 setDelegate 保活
struct SendDelegate(Retained<ProtocolObject<dyn UNUserNotificationCenterDelegate>>);
unsafe impl Send for SendDelegate {}
unsafe impl Sync for SendDelegate {}
static DELEGATE: OnceLock<SendDelegate> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    struct SparNotifDelegate;

    unsafe impl NSObjectProtocol for SparNotifDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for SparNotifDelegate {
        // 应用在前台（面板聚焦也算）时系统询问如何展示；
        // 不实现的话前台期间横幅会被静默吞掉——提醒恰恰需要随时可见
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        unsafe fn will_present_notification(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            eprintln!("[notify] willPresent -> banner");
            handler.call((UNNotificationPresentationOptions::Banner | UNNotificationPresentationOptions::List,));
        }

        // 用户与通知横幅交互（点击 = DefaultAction）时系统回调
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        unsafe fn did_receive_response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            handler: &DynBlock<dyn Fn()>,
        ) {
            let tapped =
                response.actionIdentifier().to_string() == UNNotificationDefaultActionIdentifier.to_string();
            eprintln!("[notify] didReceive response, tapped={tapped}");
            if tapped {
                if let Some(app) = PANEL_APP.get().and_then(|m| m.lock().unwrap().clone()) {
                    // 窗口操作必须回主线程
                    let runner = app.clone();
                    let _ = runner.run_on_main_thread(move || crate::show_panel(&app));
                }
            }
            // 必须调用 completion handler 通知系统处理完毕
            handler.call(());
        }
    }
);

impl SparNotifDelegate {
    fn new() -> Retained<Self> {
        unsafe { msg_send![SparNotifDelegate::class(), new] }
    }
}

/// 是否运行在 .app bundle 内。裸二进制（tauri dev / cargo run，target/debug/ 直启）下
/// UNUserNotificationCenter 会抛 NSException（bundleProxyForCurrentProcess is nil）直接崩掉
/// 整个进程——dev 模式跳过通知初始化；打包后的正式版路径在 Spar.app/Contents/MacOS/ 下不受影响。
fn in_app_bundle() -> bool {
    std::env::current_exe()
        .map(|p| {
            p.ancestors()
                .any(|a| a.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
        })
        .unwrap_or(false)
}

/// dev 模式（非 .app bundle）下没有真通知；此查询供前端展示提示用
pub fn available() -> bool {
    in_app_bundle()
}

/// 启动时调用（主线程）：装 delegate + 请求通知授权（已授权时幂等）
pub fn init(app: AppHandle) {
    if !in_app_bundle() {
        eprintln!("[notify] skip init: not running inside an .app bundle (dev mode)");
        return;
    }
    let _ = PANEL_APP.set(Mutex::new(Some(app)));
    let center = UNUserNotificationCenter::currentNotificationCenter();

    let _ = DELEGATE.set(SendDelegate(ProtocolObject::from_retained(SparNotifDelegate::new())));
    if let Some(d) = DELEGATE.get() {
        center.setDelegate(Some(&d.0));
    }

    let handler = RcBlock::new(|granted: objc2::runtime::Bool, err: *mut NSError| {
        let granted = granted.as_bool();
        let err_msg = if err.is_null() {
            "none".to_string()
        } else {
            unsafe { (*err).to_string() }
        };
        eprintln!("[notify] authorization granted={granted} err={err_msg}");
        // 被拒时降级申请临时授权（无弹窗、静默送达通知中心）；
        // 应用前台时配合 willPresent 仍会弹横幅，后台横幅需用户在系统设置手动允许
        if !granted {
            let center = UNUserNotificationCenter::currentNotificationCenter();
            let fallback = RcBlock::new(|granted2: objc2::runtime::Bool, _err2: *mut NSError| {
                eprintln!("[notify] provisional authorization granted={}", granted2.as_bool());
            });
            center.requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert
                    | UNAuthorizationOptions::Sound
                    | UNAuthorizationOptions::Provisional,
                &fallback,
            );
        }
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
        &handler,
    );
}

/// 发一条本地通知（任意线程；内部切主线程）。`id` 建议用回合号，便于定向。
pub fn notify(app: &AppHandle, title: &str, body: &str, id: &str) {
    if !in_app_bundle() {
        eprintln!("[notify] (dev, no banner) {title} — {body}");
        return;
    }
    let title = title.to_string();
    let body = body.to_string();
    let id = id.to_string();
    let _ = app.run_on_main_thread(move || {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&title));
        content.setBody(&NSString::from_str(&body));
        content.setSound(Some(&UNNotificationSound::defaultSound()));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&id),
            &content,
            None,
        );
        let completion = RcBlock::new(|err: *mut NSError| {
            if err.is_null() {
                eprintln!("[notify] add ok");
            } else {
                unsafe { eprintln!("[notify] add FAILED: {}", (*err).to_string()) };
            }
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));
        eprintln!("[notify] scheduled '{id}'");
    });
}
