use crate::constants;
use crate::game::game_data::GameData;
use crate::packet::{TcpPacket, UdpPacket};
use crate::server::packet_handler;
use crate::server::packet_handler::{TcpResponse, UdpResponse};
use bincode::{config, decode_from_slice, encode_to_vec};
use futures::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{RwLock, mpsc};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub(in crate::server) struct NetworkHandler {
    data: GameData,
    udp_tx: Arc<Sender<UdpResponse>>,
    writers: Arc<
        RwLock<HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>>,
    >,
    tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    udp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    socket: Arc<UdpSocket>,
}

impl NetworkHandler {
    pub(in crate::server) async fn new(data: GameData) -> NetworkHandler {
        let (udp_tx, udp_rx) = mpsc::channel::<UdpResponse>(constants::networking::CHANNEL_SIZE);

        let udp_tx = Arc::new(udp_tx);

        let tcp_addresses = Arc::new(RwLock::new(HashSet::<SocketAddr>::new()));
        let udp_addresses = Arc::new(RwLock::new(HashSet::<SocketAddr>::new()));

        let writers = Arc::new(RwLock::new(HashMap::<
            SocketAddr,
            Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>,
        >::new()));

        let udp_socket = Arc::new(
            UdpSocket::bind(format!("0.0.0.0:{}", constants::networking::SERVER_PORT))
                .await
                .expect("Failed to bind UDP socket"),
        );

        tokio::spawn(Self::start_tcp_server(
            data.clone(),
            tcp_addresses.clone(),
            udp_addresses.clone(),
            writers.clone(),
        ));
        Self::start_udp_server(
            data.clone(),
            udp_tx.clone(),
            udp_rx,
            udp_addresses.clone(),
            udp_socket.clone(),
        );

        NetworkHandler {
            data,
            udp_tx,
            writers,
            tcp_addresses,
            udp_addresses,
            socket: udp_socket,
        }
    }

    pub(in crate::server) fn queue_tcp_response(&self, response: TcpResponse) {
        tokio::spawn(Self::handle_tcp_response(
            response,
            self.writers.clone(),
            self.tcp_addresses.clone(),
        ));
    }

    pub(in crate::server) fn queue_udp_response(&self, response: UdpResponse) {
        tokio::spawn(Self::handle_udp_response(
            response,
            self.tcp_addresses.clone(),
            self.socket.clone(),
        ));
    }

    async fn start_tcp_server(
        data: GameData,
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        udp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
    ) {
        let socket = TcpListener::bind(format!("0.0.0.0:{}", constants::networking::SERVER_PORT))
            .await
            .unwrap();

        let mut id: u64 = 0;

        loop {
            if let Ok((stream, addr)) = socket.accept().await {
                Self::handle_tcp_client(
                    tcp_addresses.clone(),
                    udp_addresses.clone(),
                    data.clone(),
                    writers.clone(),
                    stream,
                    addr,
                    id,
                )
                .await;
                id += 1;
            }
        }
    }

    async fn handle_tcp_client(
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        udp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        data: GameData,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        stream: TcpStream,
        addr: SocketAddr,
        id: u64,
    ) {
        let (reader, writer) = stream.into_split();

        let reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let writer = FramedWrite::new(writer, LengthDelimitedCodec::new());

        writers
            .write()
            .await
            .insert(addr, Arc::new(RwLock::new(writer)));

        let (tx, rx) = mpsc::channel::<TcpResponse>(constants::networking::CHANNEL_SIZE);

        tokio::spawn(Self::start_tcp_reader(
            tcp_addresses.clone(),
            udp_addresses.clone(),
            writers.clone(),
            data,
            reader,
            tx,
            addr,
            id,
        ));
        tokio::spawn(Self::start_tcp_writer(tcp_addresses, rx, writers));
    }

    fn start_udp_server(
        data: GameData,
        tx: Arc<Sender<UdpResponse>>,
        rx: Receiver<UdpResponse>,
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        socket: Arc<UdpSocket>,
    ) {
        tokio::spawn(Self::start_udp_reader(data, socket.clone(), tx));
        tokio::spawn(Self::start_udp_writer(addresses, socket, rx));
    }

    async fn start_tcp_reader(
        tcp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        udp_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        data: GameData,
        mut reader: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        tx: Sender<TcpResponse>,
        addr: SocketAddr,
        id: u64,
    ) {
        while let Some(Ok(bytes)) = reader.next().await {
            if let Ok((packet, _)) = decode_from_slice::<TcpPacket, _>(&bytes, config::standard()) {
                let responses = packet_handler::handle_tcp_packet(
                    packet,
                    data.clone(),
                    id,
                    addr,
                    tcp_addresses.clone(),
                    udp_addresses.clone(),
                )
                .await;
                for r in responses {
                    if let Err(e) = tx.send(r).await {
                        eprintln!("TCP: failed to send response over channel: {e}");
                    }
                }
            }
        }

        tcp_addresses.write().await.remove(&addr);
        writers.write().await.remove(&addr);

        let responses = packet_handler::handle_tcp_packet(
            TcpPacket::InternalClientDisconnect,
            data.clone(),
            id,
            addr,
            tcp_addresses.clone(),
            udp_addresses.clone(),
        )
        .await;

        for r in responses {
            if let Err(e) = tx.send(r).await {
                eprintln!("TCP: failed to send response over channel: {e}");
            }
        }
    }

    async fn start_tcp_writer(
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        mut rx: Receiver<TcpResponse>,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
    ) {
        while let Some(response) = rx.recv().await {
            Self::handle_tcp_response(response, writers.clone(), addresses.clone()).await;
        }
    }

    async fn start_udp_reader(
        data: GameData,
        socket: Arc<UdpSocket>,
        tx: Arc<Sender<UdpResponse>>,
    ) {
        let mut buf = [0u8; constants::networking::BUFFER_SIZE];

        loop {
            if let Ok((size, addr)) = socket.recv_from(&mut buf).await {
                if let Ok((packet, _)) =
                    decode_from_slice::<UdpPacket, _>(&buf[..size], config::standard())
                {
                    let responses =
                        packet_handler::handle_udp_packet(packet, data.clone(), addr).await;
                    for r in responses {
                        if let Err(e) = tx.send(r).await {
                            eprintln!("UDP: failed to send TCP packet over channel: {e}");
                        }
                    }
                }
            }
        }
    }

    async fn start_udp_writer(
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        socket: Arc<UdpSocket>,
        mut rx: Receiver<UdpResponse>,
    ) {
        while let Some(response) = rx.recv().await {
            Self::handle_udp_response(response, addresses.clone(), socket.clone()).await;
        }
    }

    async fn handle_tcp_response(
        response: TcpResponse,
        writers: Arc<
            RwLock<
                HashMap<SocketAddr, Arc<RwLock<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>>>>,
            >,
        >,
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    ) {
        match response {
            TcpResponse::Broadcast {
                packet,
                sender_addr,
            } => {
                if let Ok(buf) = encode_to_vec(packet.clone(), config::standard()) {
                    let writers_to_map = writers.read().await;
                    let writers_to_send = addresses
                        .read()
                        .await
                        .iter()
                        .filter(|addr| {
                            if let Some(sender_addr) = sender_addr {
                                *addr != &sender_addr
                            } else {
                                true
                            }
                        })
                        .map(|w| writers_to_map.get(w))
                        .flatten()
                        .collect::<Vec<_>>();

                    for writer in writers_to_send {
                        if let Err(e) = writer.write().await.send(Bytes::from(buf.clone())).await {
                            eprintln!("TCP: failed to send packet over channel: {e}");
                        }
                    }
                }
            }
            TcpResponse::Reply { packet, addr } => {
                if let Ok(buf) = encode_to_vec(packet, config::standard()) {
                    if let Some(writer) = writers.read().await.get(&addr) {
                        if let Err(e) = writer.write().await.send(Bytes::from(buf)).await {
                            eprintln!("TCP: failed to send packet over channel: {e}");
                        }
                    }
                }
            }
        }
    }

    async fn handle_udp_response(
        response: UdpResponse,
        addresses: Arc<RwLock<HashSet<SocketAddr>>>,
        socket: Arc<UdpSocket>,
    ) {
        match response {
            UdpResponse::Broadcast {
                packet,
                sender_addr,
            } => {
                if let Ok(buf) = encode_to_vec(packet, config::standard()) {
                    let addresses_to_send = addresses
                        .read()
                        .await
                        .iter()
                        .cloned()
                        .filter(|addr| *addr != sender_addr)
                        .collect::<Vec<_>>();

                    for addr in addresses_to_send {
                        if let Err(e) = socket.send_to(&buf, addr).await {
                            eprintln!("UDP: failed to send packet over channel: {e}");
                        }
                    }
                }
            }
            UdpResponse::Reply { packet, addr } => {
                if let Ok(buf) = encode_to_vec(packet, config::standard()) {
                    if let Err(e) = socket.send_to(&buf, addr).await {
                        eprintln!("UDP: failed to send packet: {e}");
                    }
                }
            }
        }
    }
}
