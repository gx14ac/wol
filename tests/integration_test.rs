use std::net::UdpSocket;
use std::thread;

#[test]
fn test_magic_packet_actually_arrives_over_udp() {
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    let recv_addr = receiver.local_addr().unwrap();
    receiver
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();

    let handle = thread::spawn(move || {
        let mut buf = [0u8; 256];
        let (len, _src) = receiver.recv_from(&mut buf).unwrap();
        buf[..len].to_vec()
    });

    // Send magic packet to the receiver's address (localhost, not broadcast)
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
    let mut packet = [0xFFu8; 102];
    for i in 0..16 {
        let offset = 6 + i * 6;
        packet[offset..offset + 6].copy_from_slice(&mac);
    }
    sender.send_to(&packet, recv_addr).unwrap();

    let received = handle.join().unwrap();

    // Verify the packet arrived intact
    assert_eq!(received.len(), 102);

    // Sync stream: first 6 bytes are 0xFF
    for b in &received[..6] {
        assert_eq!(*b, 0xFF);
    }

    // MAC repeated 16 times
    for i in 0..16 {
        let offset = 6 + i * 6;
        assert_eq!(&received[offset..offset + 6], &mac);
    }
}

#[test]
fn test_cli_send_to_localhost() {
    use std::net::UdpSocket;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    let recv_addr = receiver.local_addr().unwrap();
    receiver.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

    let handle = thread::spawn(move || {
        let mut buf = [0u8; 256];
        match receiver.recv_from(&mut buf) {
            Ok((len, _)) => Some(buf[..len].to_vec()),
            Err(_) => None,
        }
    });

    // Run the actual CLI binary pointing at localhost
    let output = Command::new(env!("CARGO_BIN_EXE_wol"))
        .args([
            "send",
            "--mac",
            "11:22:33:44:55:66",
            "--broadcast",
            &recv_addr.ip().to_string(),
            "--port",
            &recv_addr.port().to_string(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "CLI failed: {}", String::from_utf8_lossy(&output.stderr));

    let received = handle.join().unwrap().expect("no packet received within timeout");

    assert_eq!(received.len(), 102);

    // Verify MAC in payload
    let expected_mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
    for i in 0..16 {
        let offset = 6 + i * 6;
        assert_eq!(&received[offset..offset + 6], &expected_mac);
    }
}
