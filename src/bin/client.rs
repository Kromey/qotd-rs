//! Client program for QotD Protocol service

use std::{
    io::Read,
    net::{TcpStream, UdpSocket},
};

use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    /// IP or hostname to connect to
    #[arg(value_name = "IP or HOSTNAME")]
    pub host: String,

    /// Port number to connect to
    #[arg(default_value_t = 17)]
    pub port: u16,

    /// Use TCP instead of UDP
    #[arg(long)]
    pub tcp: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Get the fortune from our QotD server
    let bytes = if args.tcp {
        do_tcp(args)?
    } else {
        do_udp(args)?
    };

    // Convert into a string and display the quote, propogating any conversion errors
    println!("{}", String::from_utf8(bytes)?.trim_end());

    Ok(())
}

fn do_tcp(args: Args) -> anyhow::Result<Vec<u8>> {
    // Connect to the remote server
    let mut stream = TcpStream::connect((args.host, args.port))?;

    // Read all data sent to us into a bytes Vec
    // The server will close the connection once it's sent us one quote, so this is all we need
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;

    Ok(buf)
}

fn do_udp(args: Args) -> anyhow::Result<Vec<u8>> {
    // Bind to a UDP socket; we don't care about the local address/port, any will do
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    // "Connect" to our server
    socket.connect((args.host, args.port))?;

    // Send an empty packet; anything we send is ignored, but since there's no handshake we have to start with something
    let _ = socket.send(&[0; 0])?;

    // Receive up to 512 bytes in the response - the max our server sends via UDP
    let mut buf = [0; 512];
    let len = socket.recv(&mut buf)?;

    // Convert the buffer into a Vec
    Ok(buf[..len].to_vec())
}
