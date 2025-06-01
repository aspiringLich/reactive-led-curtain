use serialport::SerialPort;
use std::time::Duration;

fn main() {
    let port_name = "/dev/ttyACM0"; // Change to your port
    let baud_rate = 115200;

    match serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(100))
        .open()
    {
        Ok(mut port) => {
            std::thread::sleep(Duration::from_secs(2));

            let msg = b"Hello Arduino!\n";
            port.write_all(msg).expect("Write failed");
            println!("Message sent!");

            println!("Starting serial monitor...");
            let mut serial_buf: Vec<u8> = vec![0; 1024];
            loop {
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            let received = String::from_utf8_lossy(&serial_buf[..bytes_read]);
                            print!("{}", received);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                    Err(e) => {
                        eprintln!("Error reading from serial port: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open port: {}", e);
        }
    }
}
