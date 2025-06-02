use std::sync::Arc;
use std::thread::{self, JoinHandle, sleep};
use std::time::Duration;

use egui::{ColorImage, mutex::Mutex};
use log;

pub struct SerialPortThread {
    _thread_handle: JoinHandle<()>,
    pub led_image: Arc<Mutex<ColorImage>>,
}

impl SerialPortThread {
    pub fn new() -> Self {
        // Create a shared ColorImage that will be updated by the main thread
        // and read by the serial port thread
        let led_image = Arc::new(Mutex::new(ColorImage::new([1, 1], egui::Color32::BLACK)));

        // Clone Arc for the thread
        let thread_led_image = Arc::clone(&led_image);

        // Spawn a thread to handle serial port communication
        let thread_handle = thread::spawn(move || {
            let _ = dbg!(serialport::available_ports());

            let port = {
                #[cfg(target_os = "linux")]
                {
                    serialport::new("/dev/ttyACM0", 500000)
                }
                #[cfg(target_os = "windows")]
                {
                    serialport::new("COM3", 500000)
                }
            };

            // Open the serial port
            let mut port = match port.timeout(Duration::from_millis(10)).open() {
                Ok(port) => port,
                Err(e) => {
                    log::error!("Failed to open serial port: {}", e);
                    return;
                }
            };

            sleep(Duration::from_millis(1000));

            // Main thread loop
            loop {
                // Acquire lock on the shared image
                let _img = thread_led_image.lock();
                let img = _img.clone();
                drop(_img);

                // Send each column of the image to the LED matrix
                for col in 0..img.width() {
                    let mut data = vec![0; img.height() * 3 + 1];
                    data[0] = col as u8;

                    for row in 0..img.height() {
                        let index = (row * 3 + 1) as usize;
                        let pixel = img[(col, row)];
                        data[index] = pixel.g();
                        data[index + 1] = pixel.r();
                        data[index + 2] = pixel.b();
                    }

                    // COBS encode the data
                    let mut encoded = cobs::encode_vec(&data);
                    encoded.push(0);

                    // Write to the serial port
                    if let Err(e) = port.write_all(&encoded) {
                        log::error!("Failed to write to serial port: {}", e);
                        sleep(Duration::from_millis(100));
                    }
                    sleep(Duration::from_micros(50));
                }

                // Read bytes from the serial port if available
                // the first half of the curtain doesnt light up without this???
                let mut buffer = [0; 512];
                match port.read(&mut buffer) {
                    Ok(0) => {} // No data available
                    Ok(bytes_read) => {
                        let data = &buffer[..bytes_read];
                        log::info!("Received {} bytes: {:?}", bytes_read, data);

                        // Try to convert to string if the data is valid UTF-8
                        if let Ok(s) = std::str::from_utf8(data) {
                            log::info!("Received string: {}", s);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => log::error!("Failed to read from serial port: {}", e),
                }

                sleep(Duration::from_millis(1));
            }
        });

        Self {
            _thread_handle: thread_handle,
            led_image,
        }
    }
}
