use crate::model::{DeckConfig, KEYS, KeyConfig};
use ab_glyph::{FontArc, PxScale};
use anyhow::{Context, Result, bail};
use hidapi::{HidApi, HidDevice};
use image::{DynamicImage, ImageBuffer, Rgb, codecs::jpeg::JpegEncoder, imageops::FilterType};
use imageproc::drawing::draw_text_mut;
use std::{
    io::Cursor,
    sync::mpsc::{Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

const VID: u16 = 0x3142;
const PID_REV1: u16 = 0x0007;
const PID_REV2: u16 = 0x0060;
const IMAGE_IDS: [u8; KEYS] = [11, 12, 13, 14, 15, 6, 7, 8, 9, 10, 1, 2, 3, 4, 5];

pub fn probe() -> Result<String> {
    let api = HidApi::new().context("iniciar hidapi")?;
    let info = api
        .device_list()
        .find(|info| {
            info.vendor_id() == VID
                && (info.product_id() == PID_REV1 || info.product_id() == PID_REV2)
                && info.usage_page() == 0xffa0
                && info.usage() == 1
        })
        .context("D6 não encontrado")?;
    let device = info
        .open_device(&api)
        .context("abrir interface HID do D6")?;
    drop(device);
    Ok(format!(
        "D6 {:04x}:{:04x}, usage {:04x}:{}, serial {}",
        VID,
        info.product_id(),
        info.usage_page(),
        info.usage(),
        info.serial_number().unwrap_or("sem serial")
    ))
}

pub fn hardware_test() -> Result<String> {
    let api = HidApi::new().context("iniciar hidapi")?;
    let info = api
        .device_list()
        .find(|info| {
            info.vendor_id() == VID
                && info.product_id() == PID_REV2
                && info.usage_page() == 0xffa0
                && info.usage() == 1
        })
        .context("D6 revisão 2 não encontrado")?;
    let device = info.open_device(&api).context("abrir D6")?;
    let key = KeyConfig {
        title: "BetterDeck".into(),
        color: [28, 48, 87],
        ..KeyConfig::default()
    };
    let jpeg = render_key(&key, true)?;
    write_crt(&device, 1024, b"DIS", &[])?;
    write_crt(&device, 1024, b"LIG", &[0, 0, 80])?;
    upload_image(&device, 1024, IMAGE_IDS[0], &jpeg)?;
    write_crt(&device, 1024, b"STP", &[])?;
    Ok(format!(
        "Imagem JPEG de {} bytes enviada à tecla 1",
        jpeg.len()
    ))
}

pub enum DeviceCommand {
    Apply(DeckConfig),
    Sleep(bool),
    Stop,
}
#[derive(Debug)]
pub enum DeviceEvent {
    Status(String),
    Press(usize),
    Error(String),
}

pub fn spawn(rx: Receiver<DeviceCommand>, tx: Sender<DeviceEvent>) {
    thread::spawn(move || {
        let mut config = DeckConfig::default();
        let mut sleeping = false;
        let mut running = true;
        while running {
            let api = match HidApi::new() {
                Ok(api) => api,
                Err(error) => {
                    let _ = tx.send(DeviceEvent::Error(format!("hidapi: {error}")));
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };
            let found = api.device_list().find(|info| {
                info.vendor_id() == VID
                    && (info.product_id() == PID_REV1 || info.product_id() == PID_REV2)
                    && info.usage_page() == 0xffa0
                    && info.usage() == 1
            });
            let Some(info) = found else {
                let _ = tx.send(DeviceEvent::Status("D6 desconectado".into()));
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(DeviceCommand::Apply(next)) => config = next,
                    Ok(DeviceCommand::Sleep(value)) => sleeping = value,
                    Ok(DeviceCommand::Stop)
                    | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => running = false,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                }
                continue;
            };
            let packet_size = if info.product_id() == PID_REV2 {
                1024
            } else {
                512
            };
            let device = match info.open_device(&api) {
                Ok(device) => device,
                Err(error) => {
                    let _ = tx.send(DeviceEvent::Error(format!(
                        "Não foi possível abrir o D6: {error}"
                    )));
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };
            let _ = tx.send(DeviceEvent::Status(format!(
                "D6 conectado ({:04x}:{:04x})",
                VID,
                info.product_id()
            )));
            if let Err(error) = initialize(&device, packet_size, &config) {
                let _ = tx.send(DeviceEvent::Error(error.to_string()));
                continue;
            }
            let mut last_heartbeat = Instant::now();
            let mut input = [0u8; 512];
            let mut held = [false; KEYS];
            loop {
                while let Ok(command) = rx.try_recv() {
                    match command {
                        DeviceCommand::Apply(next) => {
                            config = next;
                            if let Err(error) = apply(&device, packet_size, &config) {
                                let _ = tx.send(DeviceEvent::Error(error.to_string()));
                            }
                        }
                        DeviceCommand::Sleep(value) => {
                            sleeping = value;
                            let result = if value {
                                write_crt(&device, packet_size, b"HAN", &[])
                            } else {
                                write_crt(&device, packet_size, b"DIS", &[])
                                    .and_then(|_| apply(&device, packet_size, &config))
                            };
                            if let Err(error) = result {
                                let _ = tx.send(DeviceEvent::Error(error.to_string()));
                            }
                        }
                        DeviceCommand::Stop => {
                            running = false;
                            break;
                        }
                    }
                }
                if !running {
                    break;
                }
                if last_heartbeat.elapsed() >= Duration::from_secs(8) {
                    if let Err(error) = write_crt(&device, packet_size, b"CONNECT", &[]) {
                        let _ = tx.send(DeviceEvent::Error(error.to_string()));
                        break;
                    }
                    last_heartbeat = Instant::now();
                }
                match device.read_timeout(&mut input, 80) {
                    Ok(n) if n >= 11 && &input[0..3] == b"ACK" => {
                        let key = input[9] as usize;
                        if (1..=KEYS).contains(&key) {
                            let index = key - 1;
                            let down = input[10] == 1;
                            if down && !held[index] {
                                let _ = tx.send(DeviceEvent::Press(index));
                            }
                            held[index] = down;
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let _ = tx.send(DeviceEvent::Error(format!("Leitura USB: {error}")));
                        break;
                    }
                }
            }
            if running && !sleeping {
                let _ = tx.send(DeviceEvent::Status("D6 reconectando…".into()));
            }
            if running {
                thread::sleep(Duration::from_secs(1));
            }
        }
    });
}

fn initialize(device: &HidDevice, packet_size: usize, config: &DeckConfig) -> Result<()> {
    write_crt(device, packet_size, b"DIS", &[])?;
    apply(device, packet_size, config)
}

fn apply(device: &HidDevice, packet_size: usize, config: &DeckConfig) -> Result<()> {
    write_crt(device, packet_size, b"LIG", &[0, 0, config.brightness])?;
    for (index, key) in config.current_page().keys.iter().enumerate() {
        if key.title.is_empty() && key.image.is_empty() {
            write_crt(device, packet_size, b"CLE", &[0, 0, 0, IMAGE_IDS[index]])?;
        } else {
            let jpeg = render_key(key, packet_size == 1024)?;
            upload_image(device, packet_size, IMAGE_IDS[index], &jpeg)?;
        }
    }
    write_crt(device, packet_size, b"STP", &[])
}

fn write_crt(device: &HidDevice, packet_size: usize, command: &[u8], extra: &[u8]) -> Result<()> {
    write_report(device, &build_crt(packet_size, command, extra)?)
}

fn build_crt(packet_size: usize, command: &[u8], extra: &[u8]) -> Result<Vec<u8>> {
    let mut packet = vec![0u8; packet_size + 1];
    packet[1..6].copy_from_slice(b"CRT\0\0");
    let end = 6 + command.len() + extra.len();
    if end > packet.len() {
        bail!("comando USB excede o pacote");
    }
    packet[6..6 + command.len()].copy_from_slice(command);
    packet[6 + command.len()..end].copy_from_slice(extra);
    Ok(packet)
}

fn write_report(device: &HidDevice, packet: &[u8]) -> Result<()> {
    let written = device.write(packet).context("falha ao escrever no D6")?;
    if written != packet.len() {
        bail!("escrita HID incompleta: {written}/{}", packet.len());
    }
    Ok(())
}

fn upload_image(device: &HidDevice, packet_size: usize, image_id: u8, jpeg: &[u8]) -> Result<()> {
    if jpeg.len() > u16::MAX as usize {
        bail!("imagem JPEG grande demais");
    }
    let size = jpeg.len() as u16;
    write_crt(
        device,
        packet_size,
        b"BAT",
        &[0, 0, (size >> 8) as u8, size as u8, image_id],
    )?;
    for chunk in jpeg.chunks(packet_size) {
        let mut packet = vec![0u8; packet_size + 1];
        packet[1..chunk.len() + 1].copy_from_slice(chunk);
        write_report(device, &packet)?;
    }
    Ok(())
}

fn render_key(key: &KeyConfig, rotate_180: bool) -> Result<Vec<u8>> {
    let mut canvas = if key.image.is_empty() {
        ImageBuffer::from_pixel(112, 112, Rgb(key.color))
    } else {
        match image::open(&key.image) {
            Ok(image) => image
                .resize_to_fill(112, 112, FilterType::Lanczos3)
                .to_rgb8(),
            Err(_) => ImageBuffer::from_pixel(112, 112, Rgb(key.color)),
        }
    };
    if !key.title.is_empty()
        && let Ok(bytes) = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf")
        && let Ok(font) = FontArc::try_from_vec(bytes)
    {
        let lines: Vec<&str> = key.title.split_whitespace().take(3).collect();
        let white = Rgb([255, 255, 255]);
        for (line_no, line) in lines.iter().enumerate() {
            let x = 8;
            let y = 112 - (lines.len() as i32 * 22) + (line_no as i32 * 22) - 4;
            draw_text_mut(&mut canvas, white, x, y, PxScale::from(18.0), &font, line);
        }
    }
    // The D6 revision 2 rotates each panel's JPEG by 180 degrees on display.
    // Rotate the completed image, including its title, before uploading it.
    let canvas = if rotate_180 {
        image::imageops::rotate180(&canvas)
    } else {
        canvas
    };
    let image = DynamicImage::ImageRgb8(canvas);
    for quality in [80, 65, 50, 35] {
        let mut bytes = Cursor::new(Vec::new());
        JpegEncoder::new_with_quality(&mut bytes, quality).encode_image(&image)?;
        if bytes.get_ref().len() <= 10_240 {
            return Ok(bytes.into_inner());
        }
    }
    bail!("imagem JPEG excede 10 KiB")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_mapping_covers_all_panels() {
        let mut ids = IMAGE_IDS.to_vec();
        ids.sort_unstable();
        assert_eq!(ids, (1..=15).collect::<Vec<_>>());
    }
    #[test]
    fn generated_key_is_valid_jpeg() {
        let key = KeyConfig {
            title: "Teste".into(),
            ..KeyConfig::default()
        };
        let jpeg = render_key(&key, true).expect("render");
        assert!(jpeg.starts_with(&[0xff, 0xd8]));
        assert!(jpeg.len() <= 10_240);
    }
    #[test]
    fn revision_2_rotates_entire_panel() {
        let mut source = ImageBuffer::from_pixel(112, 112, Rgb([0u8, 0, 0]));
        for y in 0..32 {
            for x in 0..32 {
                source.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }
        let path =
            std::env::temp_dir().join(format!("betterdeck-orientation-{}.png", std::process::id()));
        source.save(&path).expect("save source image");
        let key = KeyConfig {
            image: path.to_string_lossy().into_owned(),
            ..KeyConfig::default()
        };
        let jpeg = render_key(&key, true).expect("render rotated JPEG");
        std::fs::remove_file(&path).expect("remove source image");
        let output = image::load_from_memory(&jpeg)
            .expect("decode JPEG")
            .to_rgb8();
        let bottom_right = output.get_pixel(104, 104).0;
        let top_left = output.get_pixel(8, 8).0;
        assert!(bottom_right[0] > 200 && bottom_right[1] < 80 && bottom_right[2] < 80);
        assert!(top_left.iter().all(|channel| *channel < 80));
    }
    #[test]
    fn crt_bat_header_matches_d6_wire_layout() {
        let packet = build_crt(1024, b"BAT", &[0, 0, 0x07, 0x14, 11]).expect("packet");
        assert_eq!(packet.len(), 1025);
        assert_eq!(
            &packet[..13],
            &[
                0, b'C', b'R', b'T', 0, 0, b'B', b'A', b'T', 0, 0, 0x07, 0x14
            ]
        );
        assert_eq!(packet[13], 11);
    }
}
