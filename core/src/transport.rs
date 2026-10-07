//! Minimal synchronous TCP transport for the Memobi wire protocol.
//!
//! This layer deliberately owns only framing and sockets. Peer/session state,
//! synchronization, and consensus remain in the node and protocol modules.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};

use crate::p2p::Message;

pub const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Protocol(crate::ProtocolError),
    FrameTooLarge,
    EmptyFrame,
}

impl From<io::Error> for TransportError {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

impl From<crate::ProtocolError> for TransportError {
    fn from(value: crate::ProtocolError) -> Self { Self::Protocol(value) }
}

pub struct TcpPeer {
    stream: TcpStream,
}

impl TcpPeer {
    pub fn connect(addr: SocketAddr) -> Result<Self, TransportError> {
        Ok(Self { stream: TcpStream::connect(addr)? })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn peer_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.stream.peer_addr()?)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.stream.local_addr()?)
    }

    pub fn set_read_timeout(&self, timeout: Option<std::time::Duration>) -> Result<(), TransportError> {
        self.stream.set_read_timeout(timeout)?;
        Ok(())
    }

    pub fn set_write_timeout(&self, timeout: Option<std::time::Duration>) -> Result<(), TransportError> {
        self.stream.set_write_timeout(timeout)?;
        Ok(())
    }

    pub fn send(&mut self, message: &Message) -> Result<(), TransportError> {
        let payload = message.encode_to_vec()?;
        if payload.is_empty() {
            return Err(TransportError::EmptyFrame);
        }
        if payload.len() > MAX_FRAME_BYTES {
            return Err(TransportError::FrameTooLarge);
        }
        let len = u32::try_from(payload.len()).map_err(|_| TransportError::FrameTooLarge)?;
        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(&payload)?;
        self.stream.flush()?;
        Ok(())
    }

    /// Gracefully close the underlying TCP connection.
    pub fn shutdown(&self) -> Result<(), TransportError> {
        self.stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }

    pub fn receive(&mut self) -> Result<Message, TransportError> {
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;
        if len == 0 {
            return Err(TransportError::EmptyFrame);
        }
        if len > MAX_FRAME_BYTES {
            return Err(TransportError::FrameTooLarge);
        }
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload)?;
        Ok(Message::decode(&payload)?)
    }
}

pub struct TcpListenerTransport {
    listener: TcpListener,
}

impl TcpListenerTransport {
    pub fn bind(addr: SocketAddr) -> Result<Self, TransportError> {
        Ok(Self { listener: TcpListener::bind(addr)? })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.listener.local_addr()?)
    }

    pub fn accept(&self) -> Result<(TcpPeer, SocketAddr), TransportError> {
        let (stream, addr) = self.listener.accept()?;
        Ok((TcpPeer::from_stream(stream), addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn tcp_transport_round_trips_wire_message() {
        let listener = TcpListenerTransport::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            peer.receive().unwrap()
        });

        let mut client = TcpPeer::connect(addr).unwrap();
        let message = Message::Ping { nonce: 123 };
        client.send(&message).unwrap();
        assert_eq!(handle.join().unwrap(), message);
    }

    #[test]
    fn tcp_transport_rejects_oversized_frame_header() {
        let listener = TcpListenerTransport::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            peer.receive()
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        let oversized = (MAX_FRAME_BYTES as u32 + 1).to_le_bytes();
        stream.write_all(&oversized).unwrap();
        assert!(matches!(handle.join().unwrap(), Err(TransportError::FrameTooLarge)));
    }
}
