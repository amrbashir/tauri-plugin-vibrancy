#![cfg(target_os = "windows")]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::ffi::c_void;
pub use windows::Win32::{
  Foundation::{BOOL, FARPROC, HWND, PSTR},
  Graphics::{
    Dwm::{
      DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE,
      DWMWINDOWATTRIBUTE, DWM_BB_ENABLE, DWM_BLURBEHIND,
    },
    Gdi::HRGN,
  },
  System::{
    LibraryLoader::{GetProcAddress, LoadLibraryA},
    SystemInformation::OSVERSIONINFOW,
  },
};

pub fn apply_acrylic(hwnd: HWND) {
  if is_win11_dwmsbt() {
    unsafe {
      DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        &(DWM_SYSTEMBACKDROP_TYPE::DWMSBT_TRANSIENTWINDOW) as *const _ as _,
        4,
      );
    }
  } else if is_supported_win10() || is_win11() {
    unsafe {
      set_window_composition_attribute(hwnd, AccentState::EnableAcrylicBlurBehind);
    }
  } else {
    eprintln!("\"apply_acrylic\" is only available on Windows 10 v1809 or newer");
  }
}
pub fn apply_blur(hwnd: HWND) {
  if is_win7() {
    let bb = DWM_BLURBEHIND {
      dwFlags: DWM_BB_ENABLE,
      fEnable: true.into(),
      hRgnBlur: HRGN::default(),
      ..Default::default()
    };
    unsafe {
      let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
    }
  } else {
    unsafe {
      set_window_composition_attribute(hwnd, AccentState::EnableBlurBehind);
    }
  }
}

pub fn apply_mica(hwnd: HWND, dark_mica: bool) {
  if is_win11() {
    unsafe {
      DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        &dark_mica as *const _ as _,
        4,
      );
    }
    if is_win11_dwmsbt() {
      unsafe {
        DwmSetWindowAttribute(
          hwnd,
          DWMWA_SYSTEMBACKDROP_TYPE,
          &(DWM_SYSTEMBACKDROP_TYPE::DWMSBT_MAINWINDOW as i32) as *const _ as _,
          4,
        );
      }
    } else {
      unsafe {
        DwmSetWindowAttribute(hwnd, DWMWA_MICA_EFFECT, &1 as *const _ as _, 4);
      }
    }
  } else {
    eprintln!("\"apply_mica\" is only available on Windows 11");
  }
}

fn get_function_impl(library: &str, function: &str) -> Option<FARPROC> {
  assert_eq!(library.chars().last(), Some('\0'));
  assert_eq!(function.chars().last(), Some('\0'));

  let module = unsafe { LoadLibraryA(PSTR(library.as_ptr() as _)) };
  if module.0 == 0 {
    return None;
  }
  Some(unsafe { GetProcAddress(module, PSTR(function.as_ptr() as _)) })
}

macro_rules! get_function {
  ($lib:expr, $func:ident) => {
    get_function_impl(concat!($lib, '\0'), concat!(stringify!($func), '\0'))
      .map(|f| unsafe { std::mem::transmute::<windows::Win32::Foundation::FARPROC, $func>(f) })
  };
}

/// Returns a tuple of (major, minor, buildnumber)
fn get_windows_ver() -> Option<(u32, u32, u32)> {
  type RtlGetVersion = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> i32;
  let handle = get_function!("ntdll.dll", RtlGetVersion);
  if let Some(rtl_get_version) = handle {
    unsafe {
      let mut vi = OSVERSIONINFOW {
        dwOSVersionInfoSize: 0,
        dwMajorVersion: 0,
        dwMinorVersion: 0,
        dwBuildNumber: 0,
        dwPlatformId: 0,
        szCSDVersion: [0; 128],
      };

      let status = (rtl_get_version)(&mut vi as _);

      if status >= 0 {
        Some((vi.dwMajorVersion, vi.dwMinorVersion, vi.dwBuildNumber))
      } else {
        None
      }
    }
  } else {
    None
  }
}

type SetWindowCompositionAttribute =
  unsafe extern "system" fn(HWND, *mut WINDOWCOMPOSITIONATTRIBDATA) -> BOOL;

type WINDOWCOMPOSITIONATTRIB = u32;

#[repr(C)]
struct ACCENT_POLICY {
  AccentState: u32,
  AccentFlags: u32,
  GradientColor: u32,
  AnimationId: u32,
}

#[repr(C)]
struct WINDOWCOMPOSITIONATTRIBDATA {
  Attrib: WINDOWCOMPOSITIONATTRIB,
  pvData: *mut c_void,
  cbData: usize,
}

pub enum AccentState {
  EnableBlurBehind = 3,
  EnableAcrylicBlurBehind = 4,
}

unsafe fn set_window_composition_attribute(hwnd: HWND, accent_state: AccentState) {
  if let Some(set_window_composition_attribute) =
    get_function!("user32.dll", SetWindowCompositionAttribute)
  {
    let mut policy = ACCENT_POLICY {
      AccentState: accent_state as _,
      AccentFlags: 2,
      GradientColor: 0x1F | 0x1F << 8 | 0x1F << 16 | 0 << 24,
      AnimationId: 0,
    };

    let mut data = WINDOWCOMPOSITIONATTRIBDATA {
      Attrib: 0x13,
      pvData: &mut policy as *mut _ as _,
      cbData: std::mem::size_of_val(&policy),
    };

    set_window_composition_attribute(hwnd, &mut data as *mut _ as _);
  }
}

const DWMWA_MICA_EFFECT: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(1029i32);
const DWMWA_SYSTEMBACKDROP_TYPE: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38i32);

enum DWM_SYSTEMBACKDROP_TYPE {
  DWMSBT_MAINWINDOW = 2,      // Mica
  DWMSBT_TRANSIENTWINDOW = 3, // Acrylic
  DWMSBT_TABBEDWINDOW = 4,    // Tabbed
}

fn is_win7() -> bool {
  let v = get_windows_ver().unwrap_or_default();
  (v.0 == 6 && v.1 == 1)
}

fn is_supported_win10() -> bool {
  let v = get_windows_ver().unwrap_or_default();
  (v.2 >= 17763 && v.2 < 22000)
}
fn is_win11() -> bool {
  let v = get_windows_ver().unwrap_or_default();
  v.2 >= 22000
}
fn is_win11_dwmsbt() -> bool {
  let v = get_windows_ver().unwrap_or_default();
  v.2 >= 22523
}
