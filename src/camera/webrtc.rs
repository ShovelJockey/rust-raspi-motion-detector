use anyhow::Result;
use axum::Json;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use http::StatusCode;
use std::sync::Arc;
use tokio::{net::UdpSocket, spawn};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        APIBuilder, API,
    },
    ice_transport::{ice_connection_state::RTCIceConnectionState, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        self, configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState, sdp::session_description::RTCSessionDescription
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{
        track_local_static_rtp::TrackLocalStaticRTP, TrackLocal, TrackLocalWriter,
    },
    Error,
};

use crate::camera::camera;

pub async fn offer_handler(
    Json(offer): Json<RTCSessionDescription>,
) -> Result<Json<RTCSessionDescription>, (StatusCode, String)> {
    // camera::start_stream_rtp();
    let offer_sdp = offer.sdp.clone();
    let offer_sdp_type = offer.sdp_type.clone();
    println!("offer sdp: {offer_sdp}, sdp type: {offer_sdp_type}");
    match handle_offer(offer).await {
        Ok(answer) => Ok(Json(answer)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

fn build_api() -> API {
    let mut m = MediaEngine::default();

    m.register_default_codecs()
        .expect("register default codecs");

    let mut registry = Registry::new();

    registry =
        register_default_interceptors(registry, &mut m).expect("register default interceptors");

    APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build()
}

async fn start_writing_track(video_track: Arc<TrackLocalStaticRTP>) {
    let udp_socket = UdpSocket::bind("239.255.255.250:5004").await.unwrap();

    tokio::spawn(async move {
        let mut inbound_rtp_packet = vec![0u8; 1500]; // UDP MTU
        while let Ok((n, _)) = udp_socket.recv_from(&mut inbound_rtp_packet).await {
            if let Err(err) = video_track.write(&inbound_rtp_packet[..n]).await {
                if Error::ErrClosedPipe == err {
                    println!("The peer conn has been closed");
                } else {
                    println!("video_track write err: {err}");
                }
                return;
            }
        }
    });
}

async fn handle_offer(
    offer: RTCSessionDescription,
) -> Result<RTCSessionDescription, Box<dyn std::error::Error>> {
    let api = build_api();

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
        .await
        .expect("add track to peer connection");

    spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        Result::<()>::Ok(())
    });

    peer_conn
        .set_remote_description(offer)
        .await
        .expect("set the remote description");

    let answer = peer_conn.create_answer(None).await.expect("create answer");

    let mut gather_complete = peer_conn.gathering_complete_promise().await;

    peer_conn
        .set_local_description(answer.clone())
        .await
        .expect("set local description");

    let _ = gather_complete.recv().await;

    start_writing_track(video_track).await;

    Ok(answer)
}
