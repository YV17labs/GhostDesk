//! The Windows [`ScreenBackend`] — GDI.
//!
//! The one backend that captures in process. Linux shells out to `grim` and
//! macOS to `screencapture`, because both operating systems ship a capture
//! tool; Windows ships none, and the alternatives to GDI are a PowerShell
//! round trip per frame or a WinRT capture session with a message pump this
//! server does not have. `BitBlt` into a memory bitmap is the short path, and
//! it is the one every screen recorder on the platform takes.
//!
//! It also has the property the macOS backend lacks: `StretchBlt` scales at
//! capture time, so a quarter-size comparison frame costs a quarter of the
//! copy rather than a full-resolution capture that is then decoded, resized
//! and re-encoded. That matters on the feedback path, which captures several
//! times per action.

#![expect(
    unsafe_code,
    reason = "GDI is C; each call carries its own SAFETY note"
)]

use anyhow::{Result, bail};
use async_trait::async_trait;
use image::ImageEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use ::windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HALFTONE,
    HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject, SetBrushOrgEx, SetStretchBltMode,
    StretchBlt,
};

use super::display;
use crate::screen::{Region, ScreenBackend};

/// The [`ScreenBackend`] the host selector hands out on Windows.
pub struct GdiScreen;

#[async_trait]
impl ScreenBackend for GdiScreen {
    async fn capture_png(&self, region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>> {
        // The blit is a synchronous copy of a few megabytes and the encode is
        // CPU-bound; the feedback loop runs both several times per second. On
        // a runtime worker they would park the whole server for the duration.
        tokio::task::spawn_blocking(move || capture(region, scale)).await?
    }

    /// Windows owns its geometry: the panel is whatever hardware is attached,
    /// and the agent's coordinates have to match the pixels it is shown.
    fn geometry(&self) -> Option<(i64, i64)> {
        Some(display::size_in_pixels())
    }
}

/// Capture, scale and encode one frame.
fn capture(region: Option<Region>, scale: Option<f32>) -> Result<Vec<u8>> {
    let (width, height) = display::size_in_pixels();
    let source = region.unwrap_or(Region {
        x: 0,
        y: 0,
        width,
        height,
    });

    if source.width <= 0 || source.height <= 0 {
        bail!(
            "nothing to capture: the region is {}x{}",
            source.width,
            source.height
        );
    }

    let scaled = scale.filter(|scale| *scale < 1.0);
    let (out_width, out_height) = match scaled {
        Some(scale) => (
            ((source.width as f32 * scale).round() as i32).max(1),
            ((source.height as f32 * scale).round() as i32).max(1),
        ),
        None => (source.width as i32, source.height as i32),
    };

    let pixels = blit(source, out_width, out_height)?;
    encode_png(
        pixels,
        out_width as u32,
        out_height as u32,
        scaled.is_some(),
    )
}

/// The device contexts and the bitmap one capture needs.
///
/// A guard rather than cleanup written at each exit. Every handle is released
/// in exactly one place, a step that fails part-way through the acquisition
/// releases what it already took, and a panic between two of them no longer
/// leaks a device context for the life of the process.
struct Gdi {
    screen: HDC,
    memory: HDC,
    bitmap: HBITMAP,
}

impl Gdi {
    fn open(width: i32, height: i32) -> Result<Self> {
        // SAFETY: `None` asks for the screen's own device context.
        let screen = unsafe { GetDC(None) };
        if screen.is_invalid() {
            bail!("the window manager refused a device context for the screen");
        }

        // From here every exit runs `Drop`, which skips the handles this has
        // not reached yet.
        let mut gdi = Self {
            screen,
            memory: HDC(std::ptr::null_mut()),
            bitmap: HBITMAP(std::ptr::null_mut()),
        };

        // SAFETY: creates a memory context compatible with the screen's.
        gdi.memory = unsafe { CreateCompatibleDC(Some(screen)) };
        if gdi.memory.is_invalid() {
            bail!("could not create a memory device context for the capture");
        }

        // Compatible with the *screen* context, not the memory one: a bitmap
        // made compatible with a fresh memory context is the 1x1 monochrome
        // bitmap that context starts life holding, and every capture would
        // come back black and white.
        // SAFETY: both extents are positive.
        gdi.bitmap = unsafe { CreateCompatibleBitmap(screen, width, height) };
        if gdi.bitmap.is_invalid() {
            bail!("could not allocate a {width}x{height} capture bitmap");
        }
        Ok(gdi)
    }

    /// Select the bitmap into the memory context for the length of one blit.
    fn selected(&self) -> Selected<'_> {
        // SAFETY: both handles are live for at least the guard's lifetime.
        let previous = unsafe { SelectObject(self.memory, self.bitmap.into()) };
        Selected {
            gdi: self,
            previous,
        }
    }
}

impl Drop for Gdi {
    fn drop(&mut self) {
        // SAFETY: each handle is this struct's own, and none is used again.
        unsafe {
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap.into());
            }
            if !self.memory.is_invalid() {
                let _ = DeleteDC(self.memory);
            }
            ReleaseDC(None, self.screen);
        }
    }
}

/// The bitmap, selected into the memory context.
///
/// A scope rather than two statements: `GetDIBits` is documented to fail on a
/// bitmap still selected into a device context, so the deselection has to
/// happen before the read — and expressed this way, an edit cannot put it
/// after.
struct Selected<'a> {
    gdi: &'a Gdi,
    previous: HGDIOBJ,
}

impl Drop for Selected<'_> {
    fn drop(&mut self) {
        // SAFETY: `previous` is what `SelectObject` handed back.
        unsafe { SelectObject(self.gdi.memory, self.previous) };
    }
}

/// Copy a screen rectangle into a memory bitmap and read its pixels back.
fn blit(source: Region, out_width: i32, out_height: i32) -> Result<Vec<u8>> {
    let gdi = Gdi::open(out_width, out_height)?;
    {
        let _selected = gdi.selected();
        copy_pixels(&gdi, source, out_width, out_height)?;
    }
    read_pixels(&gdi, out_width, out_height)
}

/// Blit the source rectangle in, scaling if the output is smaller.
fn copy_pixels(gdi: &Gdi, source: Region, out_width: i32, out_height: i32) -> Result<()> {
    let (x, y) = (source.x as i32, source.y as i32);
    let (width, height) = (source.width as i32, source.height as i32);

    // `CAPTUREBLT` includes layered windows — menus, tooltips, and anything
    // drawn with transparency. Without it the agent is shown a screen with
    // the popup it just opened missing from it.
    let rop = SRCCOPY | CAPTUREBLT;

    if (out_width, out_height) == (width, height) {
        // SAFETY: both contexts are live and the rectangle is inside the
        // screen — the caller clamped it.
        return Ok(unsafe {
            BitBlt(gdi.memory, 0, 0, width, height, Some(gdi.screen), x, y, rop)
        }?);
    }

    // SAFETY: sets a mode on a live context. `HALFTONE` averages the pixels it
    // drops instead of picking one of them, which is the whole point of a
    // scaled capture: it is compared against another, and nearest-neighbour
    // would make a one-pixel change decide the answer.
    unsafe { SetStretchBltMode(gdi.memory, HALFTONE) };
    // SAFETY: documented to be required after selecting `HALFTONE`, or the
    // scaled image is offset by the brush origin.
    let _ = unsafe { SetBrushOrgEx(gdi.memory, 0, 0, None) };

    // SAFETY: as above; the destination extent is the bitmap's own.
    let stretched = unsafe {
        StretchBlt(
            gdi.memory,
            0,
            0,
            out_width,
            out_height,
            Some(gdi.screen),
            x,
            y,
            width,
            height,
            rop,
        )
    };
    if !stretched.as_bool() {
        bail!("the screen could not be copied into a {out_width}x{out_height} bitmap");
    }
    Ok(())
}

/// Read the memory bitmap back as top-down 32-bit BGRA.
fn read_pixels(gdi: &Gdi, width: i32, height: i32) -> Result<Vec<u8>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            // Negative: a DIB is bottom-up by default, and asking for the rows
            // in the order every decoder wants them costs nothing here and a
            // full reversing copy anywhere else.
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut pixels = vec![0u8; width as usize * height as usize * 4];

    // SAFETY: `info` describes exactly the buffer `pixels` provides — 32 bits
    // per pixel over `width * height` — and both outlive the call.
    let read = unsafe {
        GetDIBits(
            gdi.memory,
            gdi.bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &raw mut info,
            DIB_RGB_COLORS,
        )
    };
    if read != height {
        bail!("the capture bitmap gave back {read} of {height} rows");
    }
    Ok(pixels)
}

/// BGRA rows → PNG.
///
/// A scaled frame is encoded for speed: those bytes are never shown to
/// anyone — the caller decodes them straight back to diff two captures — so
/// spending CPU on a smaller temporary buys nothing. A full-size frame may be
/// what the agent is served, so it gets the encoder's ordinary settings.
fn encode_png(mut pixels: Vec<u8>, width: u32, height: u32, temporary: bool) -> Result<Vec<u8>> {
    // Compacted in place, into the buffer `GetDIBits` already filled. A second
    // buffer would be another six megabytes allocated and walked for a
    // full-resolution frame, and the settle loop takes up to seventeen of
    // them per call. The write index trails the read index at every pixel, so
    // one forward pass cannot overwrite a channel it has not read yet.
    let count = width as usize * height as usize;
    for pixel in 0..count {
        let (from, to) = (pixel * 4, pixel * 3);
        let (blue, green, red) = (pixels[from], pixels[from + 1], pixels[from + 2]);
        pixels[to] = red;
        pixels[to + 1] = green;
        pixels[to + 2] = blue;
    }
    pixels.truncate(count * 3);

    let mut out = std::io::Cursor::new(Vec::new());
    let encoder = if temporary {
        PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::NoFilter)
    } else {
        PngEncoder::new_with_quality(&mut out, CompressionType::Default, FilterType::Adaptive)
    };
    encoder.write_image(&pixels, width, height, image::ExtendedColorType::Rgb8)?;
    Ok(out.into_inner())
}
