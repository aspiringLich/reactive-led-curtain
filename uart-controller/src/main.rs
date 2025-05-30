use clap::Parser;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    shuffle: Option<String>
}

fn main() {
    let args = Args::parse();
    
    let ports = serialport::available_ports().expect("No ports found!");
    for p in &ports {
        println!("{}", p.port_name);
    }

    let port = &ports[0];
    println!("using {}...", port.port_name);
    let mut port = serialport::new(&port.port_name, 115_200)
        .timeout(Duration::from_millis(100))
        .open()
        .expect("Failed to open port");

    let output = [0x03, 0x10, 0x20, 0x30, 0x00];
    port.write(&output).expect("Write failed!");
}
