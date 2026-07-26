use std::{
    ffi::c_void,
    io::Cursor,
    mem::size_of,
    path::Path,
    ptr::{copy_nonoverlapping, null_mut},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HANDLE, HWND},
        Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
            SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        },
        UI::{
            Shell::{ExtractIconExW, SHDefExtractIconW},
            WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON},
        },
    },
};

const ICON_SIZE: i32 = 128;
const NATIVE_ICON_SIZE: u32 = 256;

fn wide_null(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// Extracts a high-resolution PNG from an executable, DLL, or shortcut-provided icon source.
///
/// Returning a data URL keeps the frontend independent from filesystem protocols and
/// lets the same icon render inside both the Tauri webview and browser-side tests.
pub fn extract_icon_data_url(path: &Path, index: i32) -> Option<String> {
    if !path.is_file() {
        return None;
    }

    let mut icon = HICON::default();
    let wide_path = wide_null(path);
    let native_result = unsafe {
        SHDefExtractIconW(
            PCWSTR(wide_path.as_ptr()),
            index,
            0,
            Some(&mut icon),
            None,
            NATIVE_ICON_SIZE,
        )
    };
    if native_result.is_err() || icon.is_invalid() {
        let extracted =
            unsafe { ExtractIconExW(PCWSTR(wide_path.as_ptr()), index, Some(&mut icon), None, 1) };
        if extracted == 0 || icon.is_invalid() {
            return None;
        }
    }

    let png = unsafe { render_icon(icon) };
    unsafe {
        let _ = DestroyIcon(icon);
    }
    png.map(|bytes| format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

unsafe fn render_icon(icon: HICON) -> Option<Vec<u8>> {
    let screen_dc = GetDC(HWND::default());
    if screen_dc.is_invalid() {
        return None;
    }
    let memory_dc = CreateCompatibleDC(screen_dc);
    if memory_dc.is_invalid() {
        ReleaseDC(HWND::default(), screen_dc);
        return None;
    }

    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: ICON_SIZE,
        // A negative height creates a top-down bitmap, matching image buffer order.
        biHeight: -ICON_SIZE,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let mut bits: *mut c_void = null_mut();
    let bitmap = match CreateDIBSection(
        memory_dc,
        &bitmap_info,
        DIB_RGB_COLORS,
        &mut bits,
        HANDLE::default(),
        0,
    ) {
        Ok(bitmap) => bitmap,
        Err(_) => {
            let _ = DeleteDC(memory_dc);
            ReleaseDC(HWND::default(), screen_dc);
            return None;
        }
    };
    let previous = SelectObject(memory_dc, bitmap);

    let byte_count = (ICON_SIZE * ICON_SIZE * 4) as usize;
    std::ptr::write_bytes(bits, 0, byte_count);
    let drawn = DrawIconEx(
        memory_dc, 0, 0, icon, ICON_SIZE, ICON_SIZE, 0, None, DI_NORMAL,
    )
    .is_ok();

    let mut rgba = vec![0_u8; byte_count];
    if drawn {
        copy_nonoverlapping(bits.cast::<u8>(), rgba.as_mut_ptr(), byte_count);
        // Windows DIB sections are BGRA; web images expect RGBA.
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        // Older icons sometimes omit alpha. Preserve visible pixels in that case.
        if rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) {
            for pixel in rgba.chunks_exact_mut(4) {
                if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 {
                    pixel[3] = 255;
                }
            }
        }
    }

    SelectObject(memory_dc, previous);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(memory_dc);
    ReleaseDC(HWND::default(), screen_dc);

    if !drawn {
        return None;
    }
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(ICON_SIZE as u32, ICON_SIZE as u32, rgba)?;
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, ImageFormat::Png)
        .ok()?;
    Some(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_a_windows_system_icon_as_png_data() {
        let windows = std::env::var_os("WINDIR").expect("WINDIR should exist on Windows");
        let explorer = PathBuf::from(windows).join("explorer.exe");
        let icon = extract_icon_data_url(&explorer, 0).expect("Explorer should expose an icon");
        assert!(icon.starts_with("data:image/png;base64,"));
        assert!(icon.len() > 100);
        let bytes = STANDARD
            .decode(icon.trim_start_matches("data:image/png;base64,"))
            .unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.width(), ICON_SIZE as u32);
        assert_eq!(image.height(), ICON_SIZE as u32);
    }
}
