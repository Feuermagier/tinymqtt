use std::{
    io::{Read, Write},
    net::TcpStream,
};

use tinymqtt::MqttClient;

fn main() {
    let mut client: MqttClient<1024> = MqttClient::new();

    let mut stream = TcpStream::connect(std::env::var("TINYMQTT_HOST").unwrap()).unwrap();
    stream
        .write_all(client.connect("12345", None).unwrap())
        .unwrap();
    stream.flush().unwrap();

    let mut rx_bytes = [0; 1024];
    let len = stream.read(&mut rx_bytes).unwrap();
    client
        .receive_packet(&rx_bytes[..len], |client, topic, data| {
            println!("Received: {:?} {:?}", topic, std::str::from_utf8(data));
        })
        .unwrap();

    stream
        .write_all(client.publish("gots/test", b"test").unwrap())
        .unwrap();

    stream
        .write_all(client.subscribe("zigbee2mqtt/Power Plug 3").unwrap())
        .unwrap();

    loop {
        let len = stream.read(&mut rx_bytes).unwrap();
        client
            .receive_packet(&rx_bytes[..len], |client, topic, data| {
                println!("Received: {:?} {:?}", topic, std::str::from_utf8(data));
            })
            .unwrap();
    }
}
