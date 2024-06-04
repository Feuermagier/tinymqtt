#![no_std]

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use self::flags::Flags;
use self::reader_writer::{MqttMessageReader, MqttMessageWriter};

pub mod flags;
pub mod reader_writer;

pub type MqttResult<T> = Result<T, MqttError>;

pub struct MqttClient<const N: usize> {
    state: MqttState,
    packet_counter: PacketIdCounter,

    construct_buffer: [u8; N],
    message_buffer: [u8; N],
}

impl<const N: usize> MqttClient<N> {
    pub fn new() -> Self {
        Self {
            state: MqttState::Disconnected,
            packet_counter: PacketIdCounter::new(),
            construct_buffer: [0; N],
            message_buffer: [0; N],
        }
    }

    pub fn connect(
        &mut self,
        client_id: &str,
        username_password: Option<(&str, &str)>,
    ) -> MqttResult<&[u8]> {
        let mut writer = MqttMessageWriter::new(&mut self.construct_buffer);

        // Connect flags
        let mut flags = Flags::zero();
        flags.set(1); // clean start
        if username_password.is_some() {
            flags.set(6).set(7); // user name, password
        }

        // variable header
        writer.write_string("MQTT");
        writer.write_u8(0x05); // Protocol version
        writer.write_flags(flags); // Connect flags
        writer.write_u16(0); // Keep alive turned off
        writer.write_u8(0); // No properties

        // payload
        writer.write_string(client_id);

        if let Some((username, password)) = username_password {
            writer.write_string(username);
            writer.write_string(password);
        }

        self.state = MqttState::Connecting;

        let len = writer.len();
        self.write_packet(ControlPacketType::CONNECT, Flags::zero(), len)
    }

    pub fn publish(&mut self, topic: &str, payload: &[u8]) -> Result<&[u8], MqttError> {
        self.assert_state(MqttState::Connected)?; // Check if the client is connected

        let mut writer = MqttMessageWriter::new(&mut self.construct_buffer);

        // variable header
        writer.write_string(topic);
        writer.write_u16(0); // Packet identifier
        writer.write_u8(0); // No properties

        // payload
        writer.write_bytes_raw(payload);

        let len = writer.len();
        self.write_packet(ControlPacketType::PUBLISH, Flags::zero(), len)
    }

    pub fn subscribe(&mut self, topic_filter: &str) -> Result<&[u8], MqttError> {
        self.assert_state(MqttState::Connected)?; // Check if the client is connected

        let mut writer = MqttMessageWriter::new(&mut self.construct_buffer);

        // variable header
        writer.write_u16(self.packet_counter.next()); // Packet identifier
        writer.write_u8(0); // No properties

        // payload
        writer.write_string(topic_filter);
        writer.write_flags(Flags::zero()); // Subscription Options (with maximum QoS 0)

        let len = writer.len();
        self.write_packet(ControlPacketType::SUBSCRIBE, Flags::new(0b0010), len)
    }

    pub fn unsubscribe(&mut self, topic_filter: &str) -> Result<&[u8], MqttError> {
        self.assert_state(MqttState::Connected)?; // Check if the client is connected

        let mut writer = MqttMessageWriter::new(&mut self.construct_buffer);

        // variable header
        writer.write_u16(self.packet_counter.next()); // Packet identifier
        writer.write_u8(0); // No properties

        // payload
        writer.write_string(topic_filter);

        let len = writer.len();
        self.write_packet(ControlPacketType::UNSUBSCRIBE, Flags::new(0b0010), len)
    }

    pub fn receive_packet(
        &mut self,
        packet: &[u8],
        mut on_publish_rec: impl FnMut(&mut Self, &str, &[u8]) -> (),
    ) -> Result<MqttState, MqttError> {
        let mut reader = MqttMessageReader::new(packet);

        while reader.remaining() > 0 {
            // Parse fixed header
            let fixed_header = reader.read_u8();
            let ty = ControlPacketType::from_u8(fixed_header >> 4).unwrap();
            let _fixed_header_flags = Flags::new(fixed_header & 0x0F);
            let remaining_length = reader.read_variable_int() as usize;
            reader.mark(); // Remember start of packet content so we can skip it later

            match ty {
                ControlPacketType::CONNACK => {
                    let _connect_ack = reader.read_u8();
                    let reason_code = reader.read_u8();
                    if reason_code != 0 {
                        self.state = MqttState::Disconnected;
                        return Err(MqttError::ConnectionRefused);
                    }
                    self.state = MqttState::Connected;
                }
                ControlPacketType::SUBACK => {
                    // Nothing to do here
                }
                ControlPacketType::UNSUBACK => {
                    // Nothing to do here
                }
                ControlPacketType::PUBLISH => {
                    let topic = reader.read_string();
                    let property_length = reader.read_variable_int() as usize;
                    reader.skip(property_length); // We don't care about properties
                    let payload_length = remaining_length - reader.distance_from_mark();
                    let payload = reader.read_bytes_raw(payload_length);
                    on_publish_rec(self, topic, payload);
                }
                ControlPacketType::DISCONNECT => {
                    self.state = MqttState::Disconnected;
                    return Err(MqttError::Disconnected);
                }
                _ => {
                    return Err(MqttError::InvalidPacket);
                }
            }
            reader.skip_to(remaining_length);
        }

        Ok(self.state)
    }

    #[inline(always)]
    fn assert_state(&self, state: MqttState) -> MqttResult<()> {
        if self.state != state {
            Err(MqttError::InvalidState)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn write_packet(
        &mut self,
        ty: ControlPacketType,
        flags: Flags,
        payload_len: usize,
    ) -> MqttResult<&[u8]> {
        let mut writer = MqttMessageWriter::new(&mut self.message_buffer);
        writer.write_u8((ty as u8) << 4 | flags.value);
        writer.write_variable_int(payload_len as u32);
        writer.write_bytes_raw(&self.construct_buffer[..payload_len]);
        let len = writer.len();
        Ok(&self.message_buffer[..len])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug)]
pub enum MqttError {
    InvalidState,
    InvalidPacket,
    ConnectionRefused,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttQoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum MqttProperty {
    PayloadFormatIndicator = 0x01,
    MessageExpiryInterval = 0x02,
    ContentType = 0x03,
    ResponseTopic = 0x08,
    CorrelationData = 0x09,
    SubscriptionIdentifier = 0x0B,
    ReasonString = 0x1F,
}

struct PacketIdCounter {
    counter: u16,
}

impl PacketIdCounter {
    pub fn new() -> Self {
        Self { counter: 1 }
    }

    pub fn next(&mut self) -> u16 {
        let id = self.counter;
        self.counter = self.counter.wrapping_add(1).max(1);
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum ControlPacketType {
    CONNECT = 1,
    CONNACK = 2,
    PUBLISH = 3,
    PUBACK = 4,
    PUBREC = 5,
    PUBREL = 6,
    PUBCOMP = 7,
    SUBSCRIBE = 8,
    SUBACK = 9,
    UNSUBSCRIBE = 10,
    UNSUBACK = 11,
    PINGREQ = 12,
    PINGRESP = 13,
    DISCONNECT = 14,
    AUTH = 15,
}
