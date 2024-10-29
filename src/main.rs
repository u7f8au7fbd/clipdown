#![cfg_attr(windows, windows_subsystem = "windows")]
use arboard::Clipboard;
use chrono::Local;
use image::ImageBuffer;
use rdev::{listen, Event, EventType, Key};
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use toml;
use winrt_notification::Toast;

#[derive(Default)]
struct HotkeyState {
    enter_pressed: AtomicBool,
    t_pressed: AtomicBool,
}

fn main() {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title("ClipDown起動")
        .text1("保存先: Downloadフォルダ")
        .text2("Ctrl + Alt + Sでクリップボードの内容を保存")
        .show()
        .expect("通知の表示に失敗しました");

    let hotkey_state = Arc::new(HotkeyState::default());
    let listener_state = Arc::clone(&hotkey_state);
    thread::spawn(move || {
        listen(move |event| handle_event(event, &listener_state))
            .expect("リスナーの起動に失敗しました");
    });

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn handle_event(event: Event, hotkey_state: &Arc<HotkeyState>) {
    match event.event_type {
        EventType::KeyPress(Key::ControlLeft) => {
            hotkey_state.enter_pressed.store(true, Ordering::SeqCst)
        }
        EventType::KeyRelease(Key::ControlLeft) => {
            hotkey_state.enter_pressed.store(false, Ordering::SeqCst)
        }
        EventType::KeyPress(Key::Alt) => hotkey_state.t_pressed.store(true, Ordering::SeqCst),
        EventType::KeyRelease(Key::Alt) => hotkey_state.t_pressed.store(false, Ordering::SeqCst),
        EventType::KeyPress(Key::KeyS) => {
            if hotkey_state.enter_pressed.load(Ordering::SeqCst)
                && hotkey_state.t_pressed.load(Ordering::SeqCst)
            {
                save_clipboard_content();
            }
        }
        _ => (),
    }
}

fn is_valid_json(input: &str) -> bool {
    serde_json::from_str::<Value>(input).is_ok()
}

fn format_json(input: &str) -> String {
    serde_json::to_string_pretty(&serde_json::from_str::<Value>(input).unwrap()).unwrap()
}

fn is_valid_toml(input: &str) -> bool {
    toml::from_str::<toml::Value>(input).is_ok()
}

fn save_clipboard_content() {
    // クリップボードにアクセス
    let mut clipboard = Clipboard::new().expect("クリップボードへのアクセスに失敗しました");
    let download_dir = env::var("USERPROFILE").unwrap() + "\\Downloads\\";

    if let Ok(image) = clipboard.get_image() {
        // 現在の日時を取得してファイル名を作成
        let now = Local::now();
        let timestamp = now.format("Img_%Y-%m-%d_%H-%M-%S-%3f").to_string();
        let output_path = format!("{}{}.png", download_dir, timestamp);

        // 画像バッファを作成して保存
        let buffer: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .expect("画像バッファの作成に失敗しました");
        buffer.save(&output_path).expect("画像の保存に失敗しました");

        // 通知表示
        show_notification(
            "保存完了",
            &format!("画像が保存されました: {}", output_path),
        );
    } else if let Ok(text) = clipboard.get_text() {
        // 現在の日時を取得
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d_%H-%M-%S-%3f").to_string();

        // JSONまたはTOMLの場合、日時を含むファイル名で保存する
        let (output_path, content) = if is_valid_json(&text) {
            (
                download_dir.clone() + &format!("Json_{}.json", timestamp),
                format_json(&text),
            )
        } else if is_valid_toml(&text) {
            (
                download_dir.clone() + &format!("Toml_{}.toml", timestamp),
                text,
            )
        } else {
            // 通常のテキストの場合
            (download_dir + &format!("Text_{}.txt", timestamp), text)
        };

        // ファイルの作成と書き込み
        let mut file = File::create(&output_path).expect("ファイルの作成に失敗しました");
        file.write_all(content.as_bytes())
            .expect("書き込みに失敗しました");

        // 通知表示
        show_notification(
            "保存完了",
            &format!("テキストが保存されました: {}", output_path),
        );
    } else {
        // クリップボードに画像もテキストも含まれていない場合の通知
        show_notification(
            "保存失敗",
            "クリップボードには画像もテキストも含まれていません",
        );
    }
}

fn show_notification(title: &str, text: &str) {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(text)
        .show()
        .expect("通知の表示に失敗しました");
}
