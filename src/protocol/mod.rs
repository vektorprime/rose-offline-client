use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolClientError {
    #[error("client initiated disconnect")]
    ClientInitiatedDisconnect,
}

/// Maximum consecutive malformed packets tolerated before the stream is
/// considered unrecoverable (desynchronized) and the connection is closed.
pub const MAX_MALFORMED_PACKETS: u32 = 8;

#[async_trait]
pub trait ProtocolClient {
    async fn run_connection(&mut self) -> Result<(), anyhow::Error>;
}

#[macro_export]
macro_rules! implement_protocol_client {
    ( $x:ident ) => {
        #[async_trait]
        impl ProtocolClient for $x {
            async fn run_connection(&mut self) -> Result<(), anyhow::Error> {
                let socket = TcpStream::connect(&self.server_address).await?;
                let mut connection = Connection::new(socket, self.packet_codec.as_ref());
                let mut malformed_packets = 0u32;

                loop {
                    tokio::select! {
                        packet = connection.read_packet() => {
                            match packet {
                                Ok(packet) => {
                                    match self.handle_packet(&packet).await {
                                        Ok(_) => {
                                            malformed_packets = 0;
                                        },
                                        Err(error) => {
                                            // Malformed payload: framing is intact, skip the packet.
                                            malformed_packets += 1;
                                            log::warn!("Error {} handling packet [{:03X}] {:02x?}", error, packet.command, &packet.data[..]);
                                            if malformed_packets >= crate::protocol::MAX_MALFORMED_PACKETS {
                                                return Err(error);
                                            }
                                        },
                                    }
                                },
                                Err(error) => {
                                    if matches!(error.downcast_ref::<rose_network_common::ConnectionError>(), Some(rose_network_common::ConnectionError::DecryptBodyFailed)) {
                                        // Malformed frame: the buffer was advanced past the bad
                                        // frame, so skip it and resync to the next packet.
                                        malformed_packets += 1;
                                        log::warn!("Skipping malformed packet: {error}");
                                        if malformed_packets >= crate::protocol::MAX_MALFORMED_PACKETS {
                                            return Err(error);
                                        }
                                    } else {
                                        return Err(error);
                                    }
                                }
                            }
                        },
                        server_message = self.client_message_rx.recv() => {
                            if let Some(message) = server_message {
                                self.handle_client_message(&mut connection, message).await?;
                            } else {
                                return Err(ProtocolClientError::ClientInitiatedDisconnect.into());
                            }
                        }
                    };
                }
            }
        }
    };
}

pub mod irose;
