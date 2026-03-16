use crate::client::packet_handler;
use crate::constants;
use crate::game::game_data::GameData;
use crate::packet::{TcpPacket, UdpPacket};
use bincode::{config, decode_from_slice, encode_into_slice, encode_to_vec};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub(in crate::client) struct NetworkHandler {
    tcp_tx: Arc<Sender<TcpPacket>>,
    udp_tx: Arc<Sender<UdpPacket>>,
    shutdown_tx: broadcast::Sender<()>,
    udp_port: u16,
}

impl NetworkHandler {
    pub(in crate::client) async fn new(
        data: GameData,
        addr: SocketAddr,
        username: String,
    ) -> Result<NetworkHandler, String> {
        let (tcp_tx, tcp_rx) = mpsc::channel::<TcpPacket>(constants::networking::CHANNEL_SIZE);
        let (udp_tx, udp_rx) = mpsc::channel::<UdpPacket>(constants::networking::CHANNEL_SIZE);
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        let tcp_tx = Arc::new(tcp_tx);
        let udp_tx = Arc::new(udp_tx);

        let socket = Arc::new(
            UdpSocket::bind("0.0.0.0:0")
                .await
                .expect("Failed to bind UDP socket"),
        );

        let _ = socket.connect(addr).await;

        let udp_port = socket.local_addr().unwrap().port();

        Self::start_tcp_client(
            data.clone(),
            tcp_tx.clone(),
            tcp_rx,
            addr.clone(),
            shutdown_tx.subscribe(),
            username,
        )
        .await?;

        tokio::spawn(Self::start_udp_client(
            data.clone(),
            udp_tx.clone(),
            udp_rx,
            socket,
            shutdown_tx.subscribe(),
        ));

        Ok(NetworkHandler {
            tcp_tx,
            udp_tx,
            shutdown_tx,
            udp_port,
        })
    }

    pub(in crate::client) fn queue_tcp_packet(&self, packet: TcpPacket) {
        let tx = self.tcp_tx.clone();
        tokio::spawn(async move {
            tx.send(packet)
                .await
                .expect("Failed to send TCP packet over channel");
        });
    }

    pub(in crate::client) fn queue_udp_packet(&self, packet: UdpPacket) {
        let tx = self.udp_tx.clone();
        tokio::spawn(async move {
            tx.send(packet)
                .await
                .expect("Failed to send UDP packet over channel");
        });
    }

    pub(in crate::client) async fn shutdown(&self) {
        if let Err(e) = self.shutdown_tx.send(()) {
            eprintln!("Error sending shutdown signal: {:?}", e);
        }
    }

    async fn start_tcp_client(
        data: GameData,
        tx: Arc<Sender<TcpPacket>>,
        rx: Receiver<TcpPacket>,
        addr: SocketAddr,
        shutdown_rx: broadcast::Receiver<()>,
        username: String,
    ) -> Result<(), String> {
        let stream = timeout(
            Duration::from_millis(constants::SERVER_CONNECT_TIMEOUT_MS),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let (reader, writer) = stream.into_split();

        let reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let writer = FramedWrite::new(writer, LengthDelimitedCodec::new());

        tokio::spawn(Self::start_tcp_reader(data, reader, tx, shutdown_rx));
        tokio::spawn(Self::start_tcp_writer(writer, rx, username));

        Ok(())
    }

    async fn start_udp_client(
        data: GameData,
        tx: Arc<Sender<UdpPacket>>,
        rx: Receiver<UdpPacket>,
        socket: Arc<UdpSocket>,
        shutdown_rx: broadcast::Receiver<()>,
    ) {
        tokio::spawn(Self::start_udp_reader(
            data,
            socket.clone(),
            tx,
            shutdown_rx.resubscribe(),
        ));
        tokio::spawn(Self::start_udp_writer(socket.clone(), rx, shutdown_rx));
    }

    async fn start_tcp_reader(
        data: GameData,
        mut reader: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tx: Arc<Sender<TcpPacket>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                result = reader.next() => {
                    if let Some(Ok(bytes)) = result {
                        if let Ok((packet, _)) = decode_from_slice::<TcpPacket, _>(&bytes, config::standard()) {
                            let response = packet_handler::handle_tcp_packet(packet, data.clone()).await;
                            for r in response {
                                if let Err(e) = tx.send(r).await {
                                    eprintln!("TCP: failed to send response over channel: {e}");
                                }
                            }
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    println!("Shutting down TCP reader!");
                    break;
                }
            }
        }
    }

    async fn start_tcp_writer(
        mut writer: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        mut rx: Receiver<TcpPacket>,
        username: String,
    ) {
        let bytes = encode_to_vec(TcpPacket::JoinRequest { username }, config::standard()).unwrap();
        if let Err(e) = writer.send(Bytes::from(bytes)).await {
            eprintln!("TCP: failed to send init packet: {e}");
        }

        while let Some(packet) = rx.recv().await {
            let bytes = encode_to_vec(&packet, config::standard()).unwrap();
            if let Err(e) = writer.send(Bytes::from(bytes)).await {
                eprintln!("TCP: failed to send packet over network: {e}");
            }
        }
    }

    async fn start_udp_reader(
        data: GameData,
        socket: Arc<UdpSocket>,
        tx: Arc<Sender<UdpPacket>>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        let mut buf = [0u8; constants::networking::BUFFER_SIZE];

        loop {
            tokio::select! {
                result = socket.recv(&mut buf) => {
                    if let Ok(size) = result {
                        if let Ok((packet, _)) =
                            decode_from_slice::<UdpPacket, _>(&buf[..size], config::standard())
                        {
                            let packets =
                                packet_handler::handle_udp_packet(packet, data.clone()).await;
                            for p in packets {
                                if let Err(e) = tx.send(p).await {
                                    eprintln!("UDP: failed to send TCP packet over channel: {e}");
                                }
                            }
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    println!("Shutting down UDP reader!");
                    break;
                }
            }
        }
    }

    async fn start_udp_writer(
        socket: Arc<UdpSocket>,
        mut rx: Receiver<UdpPacket>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) {
        let mut buf = [0u8; constants::networking::BUFFER_SIZE];

        loop {
            tokio::select! {
                Some(packet) = rx.recv() => {
                    if let Ok(size) = encode_into_slice(&packet, &mut buf, config::standard()) {
                        if let Err(e) = socket.send(&buf[..size]).await {
                            eprintln!("UDP: failed to send TCP packet over channel: {e}");
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    println!("Shutting down UDP writer!");
                    break;
                }
            }
        }
    }
}
