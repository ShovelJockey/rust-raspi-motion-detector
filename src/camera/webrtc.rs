use std::sync::Arc;
use anyhow::Result;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use tokio::{net::UdpSocket, spawn};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        APIBuilder,
    },
    interceptor::registry::Registry,
    peer_connection::{configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState, sdp::session_description::RTCSessionDescription},
    ice_transport::{ice_connection_state::RTCIceConnectionState, ice_server::RTCIceServer},
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocal, TrackLocalWriter},
    Error
};

fn must_read_stdin() -> Result<String> {
    let mut line = String::new();

    std::io::stdin().read_line(&mut line)?;
    line = line.trim().to_owned();
    println!();

    Ok(line)
}

fn decode(s: &str) -> Result<String> {
    let b = BASE64_STANDARD.decode(s)?;

    let s = String::from_utf8(b)?;
    Ok(s)
}

fn encode(b: &str) -> String {

    BASE64_STANDARD.encode(b)
}

async fn setup_webrtc_conversion() {
    let mut m = MediaEngine::default();

    m.register_default_codecs()
        .expect("register default codecs");

    let mut registry = Registry::new();

    registry =
        register_default_interceptors(registry, &mut m).expect("register default interceptors");

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_conn = Arc::new(
        api.new_peer_connection(config)
            .await
            .expect("new peer connection"),
    );

    let video_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_H264.to_owned(),
            clock_rate: 90000,
            channels: 0,
            sdp_fmtp_line: "packetization-mode=1;profile-level-id=42e01f".to_owned(),
            rtcp_feedback: vec![],
        },
        "video".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    let rtp_sender = peer_conn
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await.expect("add track to peer connection");

    spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
            Result::<()>::Ok(())
        });

    let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

    let done_tx1 = done_tx.clone();
    peer_conn.on_ice_connection_state_change(Box::new(
        move |connection_state: RTCIceConnectionState| {
            println!("Connection State has changed {connection_state}");
            if connection_state == RTCIceConnectionState::Failed {
                let _ = done_tx1.try_send(());
            }
            Box::pin(async {})
        },
    ));

    let done_tx2 = done_tx.clone();

    peer_conn.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        println!("Peer Connection State has changed: {s}");

        if s == RTCPeerConnectionState::Failed {
            println!("Peer Connection has gone to failed exiting: Done forwarding");
            let _ = done_tx2.try_send(());
        }

        Box::pin(async {})
    }));

    let line = must_read_stdin().expect("failed to read signal");
    let desc_data = decode(line.as_str()).expect("failed to decode data");
    let offer = serde_json::from_str::<RTCSessionDescription>(&desc_data).expect("read desc data as rtc sess desc");

    peer_conn.set_remote_description(offer).await.expect("failed to set remote description");

    let answer = peer_conn.create_answer(None).await.expect("created answer");

    let mut gather_complete = peer_conn.gathering_complete_promise().await;

    peer_conn.set_local_description(answer).await.expect("set local description");

    let _ = gather_complete.recv().await;

    if let Some(local_desc) = peer_conn.local_description().await {
        let json_str = serde_json::to_string(&local_desc).expect("local desc to json str");
        let b64 = encode(&json_str);
        println!("{b64}");
    } else {
        println!("generate local_description failed!");
    }

    let udp_socket = UdpSocket::bind("0.0.0.0:8080").await.unwrap();

    let done_tx3 = done_tx.clone();

    tokio::spawn(async move {
        let mut inbound_rtp_packet = vec![0u8; 1600]; // UDP MTU
        while let Ok((n, _)) = udp_socket.recv_from(&mut inbound_rtp_packet).await {
            if let Err(err) = video_track.write(&inbound_rtp_packet[..n]).await {
                if Error::ErrClosedPipe == err {
                    println!("The peer conn has been closed");
                } else {
                    println!("video_track write err: {err}");
                }
                let _ = done_tx3.try_send(());
                return;
            }
        }
    });

    println!("Press ctrl-c to stop");
    tokio::select! {
        _ = done_rx.recv() => {
            println!("received done signal!");
        }
        _ = tokio::signal::ctrl_c() => {
            println!();
        }
    };

    peer_conn.close().await.expect("close peer conn");
}
