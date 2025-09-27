use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, env::var};
use tokio::{net::UdpSocket, spawn, sync::Mutex};
use tracing::{debug, error, info};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        APIBuilder, API,
    },
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_server::RTCIceServer,
    },
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{
        track_local_static_rtp::TrackLocalStaticRTP, TrackLocal, TrackLocalWriter,
    },
    Error,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    Offer { sdp: String },
    IceCandidate { candidate: String },
}

#[derive(Serialize, Deserialize)]
struct CandidateFormat {
    #[serde(rename = "type")]
    data_type: String,
    candidate: RTCIceCandidateInit,
}

pub async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket))
}

async fn handle_socket(socket: WebSocket) {
    let (sender, mut reciever) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    let api = build_api();

    let unparsed_urls = var("TURN_URL").unwrap();
    let username = var("TURN_USER").unwrap();
    let password = var("TURN_PASS").unwrap();

    let turn_urls = unparsed_urls.split(",").map(|s| s.to_string()).collect();

    let config = RTCConfiguration {
        ice_servers: vec![
            RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            },
            RTCIceServer {
                urls: turn_urls,
                username: username,
                credential: password
            }
        ],
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
            ..Default::default()
        },
        "video".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    let rtp_sender = peer_conn
        .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .expect("add track to peer connection");

    let buff_reader = spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        Result::<()>::Ok(())
    });

    let udp_socket = UdpSocket::bind("127.0.0.1:5004").await.unwrap();

    let track_writer = spawn(async move {
        let mut inbound_rtp_packet = vec![0u8; 1500]; // UDP MTU
        while let Ok((n, _)) = udp_socket.recv_from(&mut inbound_rtp_packet).await {
            debug!("packet length: {n}");
            if let Err(err) = video_track.write(&inbound_rtp_packet[..n]).await {
                if Error::ErrClosedPipe == err {
                    error!("The peer conn has been closed");
                } else {
                    error!("video_track write err: {err}");
                }
                return;
            }
        }
    });

    let ice_sender = sender.clone();
    peer_conn.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        let ice_sender_clone = ice_sender.clone();
        Box::pin(async move {
            if let Some(candidate) = candidate {
                info!("New ICE candidate: {:?}", candidate);
                let candidate_json = candidate
                    .to_json()
                    .expect("Candidate to be json serialised");
                let formatted_candidate = CandidateFormat {
                    data_type: "ice_candidate".to_string(),
                    candidate: candidate_json,
                };
                let candidate_string = serde_json::to_string(&formatted_candidate)
                    .expect("candidate json to be string serialised");
                let mut sender_lock = ice_sender_clone.lock().await;
                sender_lock
                    .send(Message::Text(candidate_string.into()))
                    .await
                    .expect("send new ice candidate");
            }
        })
    }));

    while let Some(Ok(message)) = reciever.next().await {
        if let Message::Text(text) = message {
            if let Ok(client_message) = serde_json::from_str::<ClientMessage>(&text) {
                match client_message {
                    ClientMessage::Offer { sdp } => {
                        let raw_offer = sdp.clone();
                        debug!("raw offer: {raw_offer}");
                        match RTCSessionDescription::offer(sdp) {
                            Ok(offer) => {
                                peer_conn
                                    .set_remote_description(offer)
                                    .await
                                    .expect("set the remote description");

                                let answer =
                                    peer_conn.create_answer(None).await.expect("create answer");
                                peer_conn
                                    .set_local_description(answer.clone())
                                    .await
                                    .expect("local desc set");
                                let raw_answer = answer.sdp.clone();
                                debug!("raw answer: {raw_answer}");
                                let json_string_answer =
                                    serde_json::to_string(&answer).expect("answer working format");
                                let mut answer_sender = sender.lock().await;
                                answer_sender
                                    .send(Message::Text(json_string_answer.into()))
                                    .await
                                    .expect("replied with answer");
                            }
                            Err(err) => {
                                error!("Error with browser SDP, error: {err}")
                            }
                        }
                    }
                    ClientMessage::IceCandidate { candidate } => {
                        match serde_json::from_str::<RTCIceCandidateInit>(&candidate) {
                            Ok(ice_candidate) => {
                                peer_conn
                                    .add_ice_candidate(ice_candidate)
                                    .await
                                    .expect("set ice candidate");
                            }
                            Err(err) => {
                                error!("failed to parse incoming candidate string as ice candidate, err: {err}")
                            }
                        }
                    }
                }
            }
        }
    }
    info!("socket closed");
    buff_reader.abort();
    track_writer.abort();
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
