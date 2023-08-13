use std::sync::OnceLock;

use windows::Win32::{
    Foundation::{CloseHandle, BOOL, HMODULE, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM},
    System::{
        ProcessStatus::GetProcessImageFileNameA,
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
    },
    UI::{
        Input::KeyboardAndMouse::{VK_NUMLOCK, VIRTUAL_KEY, VK_PAUSE},
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageA, EnumWindows, GetMessageA, GetWindowThreadProcessId,
                SetWindowsHookExA, TranslateMessage, HHOOK, KBDLLHOOKSTRUCT, MSG,
            MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APPCOMMAND, WM_KEYDOWN, WM_XBUTTONDOWN, SendMessageA,
        },
    },
};

static SPOTIFY_HWND: OnceLock<HWND> = OnceLock::new();

pub enum SpotifyCommand {
    Mute = 0x80000,
    VolumeDown = 0x90000,
    VolumeUp = 0xA0000,
    Next = 0xB0000,
    Previous = 0xC0000,
    Stop = 0xD0000,
    PlayPause = 0xE0000,
}

extern "system" fn mouse_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if wparam.0 == WM_XBUTTONDOWN as usize {
            let event = &*(lparam.0 as *mut MSLLHOOKSTRUCT);
            let button = (event.mouseData >> 16) as i16;

            if button == 1 {
                send_message(SpotifyCommand::PlayPause);
            }
        }

        CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
    }
}

extern "system" fn keyboard_hook(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if wparam.0 == WM_KEYDOWN as usize {
            let event = &*(lparam.0 as *mut KBDLLHOOKSTRUCT);

            match VIRTUAL_KEY(event.vkCode as u16) {
                VK_NUMLOCK => send_message(SpotifyCommand::Next),
                VK_PAUSE => send_message(SpotifyCommand::PlayPause),
                _ => (),
            }
        }

        CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
    }
}

fn send_message(command: SpotifyCommand) {
    unsafe {
        let hwnd = SPOTIFY_HWND.get().unwrap();

        SendMessageA(
            *hwnd,
            WM_APPCOMMAND,
            WPARAM::default(),
            LPARAM(command as isize),
        );
    }
}

unsafe fn application_loop() -> windows::core::Result<()> {
    let mut lpmsg = MSG::default(); 
    while GetMessageA(&mut lpmsg, HWND::default(), 0, 0).as_bool() {
        TranslateMessage(&mut lpmsg);
        DispatchMessageA(&mut lpmsg);
    }

    Ok(())
}

unsafe fn get_spotify() -> Option<HWND> {
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let mut buffer = [0u8; MAX_PATH as usize];
        let mut process_id = 0u32;
        let len: u32;

        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        let handle = match OpenProcess(PROCESS_QUERY_INFORMATION, false, process_id) {
            Ok(handle) => handle,
            Err(_) => return true.into(),
        };

        len = GetProcessImageFileNameA(handle, &mut buffer);
        CloseHandle(handle);

        if len <= 0 {
            return true.into();
        }

        let str = String::from_utf8_lossy(&buffer[..len as usize]);

        if str.ends_with("Spotify.exe") {
            let output = lparam.0 as *mut HWND;
            *output = hwnd;

            println!("Found Spotify: {}", str);
            false.into()
        } else {
            true.into()
        }
    }

    let mut handle = HWND::default();
    let ptr = &mut handle as *mut HWND;
    match !EnumWindows(Some(enum_callback), LPARAM(ptr as isize)).as_bool() {
        true => Some(handle),
        false => None,
    }
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let hwnd = get_spotify().expect("Failed to find Spotify window handle");

        SPOTIFY_HWND.set(hwnd).unwrap();

        SetWindowsHookExA(WH_MOUSE_LL, Some(mouse_hook), HMODULE::default(), 0)?;
        SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), HMODULE::default(), 0)?;

        application_loop()
    }
}
