#![windows_subsystem = "windows"]
// 必要なモジュールのインポート
use arboard::Clipboard;
use image::ImageBuffer;
use rdev::{listen, Event, EventType, Key};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use winrt_notification::Toast; // Windowsネイティブ通知用のモジュール

// グローバルなホットキーの状態を追跡
#[derive(Default)]
struct HotkeyState {
    enter_pressed: bool,
    t_pressed: bool,
}

fn main() {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title("ClipDown起動")
        .text1("保存先: Downloadフォルダ")
        .text2("Ctrl + Alt + Sでクリップボードの内容を保存")
        .show()
        .expect("通知の表示に失敗しました");
    let hotkey_state = Arc::new(Mutex::new(HotkeyState::default()));

    // ホットキーのリスナーをスレッドで起動
    let listener_state = Arc::clone(&hotkey_state);
    thread::spawn(move || {
        listen(move |event| handle_event(event, &listener_state))
            .expect("リスナーの起動に失敗しました");
    });

    // メインスレッドを永続化
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

// イベントハンドラ
fn handle_event(event: Event, hotkey_state: &Arc<Mutex<HotkeyState>>) {
    let mut state = hotkey_state.lock().unwrap();

    match event.event_type {
        EventType::KeyPress(Key::ControlLeft) => state.enter_pressed = true,
        EventType::KeyRelease(Key::ControlLeft) => state.enter_pressed = false,
        EventType::KeyPress(Key::Alt) => state.t_pressed = true,
        EventType::KeyRelease(Key::Alt) => state.t_pressed = false,
        EventType::KeyPress(Key::KeyS) => {
            if state.enter_pressed && state.t_pressed {
                save_clipboard_content();
            }
        }
        _ => (),
    }
}

// パスに使えない文字を除去するヘルパー関数
fn sanitize_filename(input: &str) -> String {
    // 使用不可な文字を除外し、文字列の長さを12文字に制限
    input
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == ' ' || c == '_')
        .take(12)
        .collect()
}

// クリップボードから画像または文字列を保存する関数
fn save_clipboard_content() {
    let mut clipboard = Clipboard::new().expect("クリップボードへのアクセスに失敗しました");

    // ダウンロードフォルダのパスを取得
    let download_dir = env::var("USERPROFILE").unwrap() + "\\Downloads\\";

    // クリップボードから画像取得を試みる
    if let Ok(image) = clipboard.get_image() {
        // 画像データのハッシュを計算してファイル名に使用
        let mut hasher = DefaultHasher::new();
        image.bytes.hash(&mut hasher);
        let hash = hasher.finish();
        let output_path = Path::new(&download_dir).join(format!("cb-img_{}.png", hash));

        // 画像データを保存
        let buffer: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
        .expect("画像バッファの作成に失敗しました");

        buffer.save(&output_path).expect("画像の保存に失敗しました");

        // Windowsネイティブ通知で画像保存完了を通知
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title("保存完了")
            .text1("画像がDownloadフォルダに保存されました")
            .show()
            .expect("通知の表示に失敗しました");
    } else if let Ok(text) = clipboard.get_text() {
        // テキストデータの先頭12文字を取得し、ファイル名に適用可能な文字のみで構成
        let sanitized_text = sanitize_filename(&text);
        let output_path = Path::new(&download_dir).join(format!("{}_text.txt", sanitized_text));

        // テキストデータを保存
        let mut file = File::create(&output_path).expect("テキストファイルの作成に失敗しました");
        file.write_all(text.as_bytes())
            .expect("テキストの書き込みに失敗しました");

        // Windowsネイティブ通知でテキスト保存完了を通知
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title("保存完了")
            .text1("テキストがDownloadフォルダに保存されました")
            .show()
            .expect("通知の表示に失敗しました");
    } else {
        // Windowsネイティブ通知でクリップボードが空であることを通知
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title("保存失敗")
            .text1("クリップボードには画像もテキストも含まれていません")
            .show()
            .expect("通知の表示に失敗しました");
    }
}
