use anyhow::Result;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{env::var, sync::{Arc, RwLock}};
use tokio::{net::UdpSocket, spawn, sync::Mutex, task::JoinHandle};
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
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription, RTCPeerConnection,
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

pub struct WebrtcState {
    api: API,
    video_track: Arc<TrackLocalStaticRTP>,
    video_task: Arc<RwLock<Option<JoinHandle<()>>>>
}

impl WebrtcState {
    pub fn new() -> WebrtcState {
        WebrtcState {
            api: WebrtcState::build_api(),
            video_track: Arc::new(WebrtcState::create_video_track()),
            video_task: Arc::new(RwLock::new(None))
        }
    }

    pub async fn new_peer_connection(&self) -> Arc<RTCPeerConnection> {
        Arc::new(
            self.api.new_peer_connection(WebrtcState::create_config())
                .await
                .expect("new peer connection"),
        )
    }

    fn start_video_task(&self) {
        if self.video_task.read().unwrap().is_none() {
            debug!("Starting video writer task");
            *self.video_task.write().unwrap() = Some(self.create_video_task());
        }
        debug!("Video task already started");
    }

    fn create_video_task(&self) -> JoinHandle<()> {
        let task_track = self.video_track.clone();
        debug!("start video task");
        spawn(async move {
            let mut inbound_rtp_packet = BytesMut::zeroed(1500);
            let udp_socket = UdpSocket::bind("127.0.0.1:5004").await.unwrap();
            debug!("bound socket");
            while let Ok((n, _)) = udp_socket.recv_from(&mut inbound_rtp_packet).await {
                debug!("packet length: {n}");
                if let Err(err) = task_track.write(&inbound_rtp_packet[..n]).await {
                    if Error::ErrClosedPipe == err {
                        error!("The peer conn has been closed");
                    } else {
                        error!("video_track write err: {err}");
                    }
                    return;
                }
            }
        })
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

    fn create_video_track() -> TrackLocalStaticRTP {
        TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_H264.to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            "webrtc-rs".to_owned(),
        )
    }

    fn create_config() -> RTCConfiguration {
        let turn_url = var("TURN_URL").unwrap();
        let username = var("TURN_USER").unwrap();
        let password = var("TURN_PASS").unwrap();
        RTCConfiguration {
            ice_servers: vec![
                RTCIceServer {
                    urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                    ..Default::default()
                },
                RTCIceServer {
                    urls: vec![turn_url],
                    username: username,
                    credential: password,
                },
            ],
            ..Default::default()
        }
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, webrtc_state: State<Arc<WebrtcState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, webrtc_state))
}

async fn handle_socket(socket: WebSocket, webrtc_state: State<Arc<WebrtcState>>) {
    webrtc_state.start_video_task();

    let (sender, mut reciever) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    let peer_conn = webrtc_state.new_peer_connection().await;

    let rtp_sender = peer_conn
        .add_track(Arc::clone(&webrtc_state.video_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
        .expect("add track to peer connection");

    let buff_reader = spawn(async move {
        let mut rtcp_buf = BytesMut::with_capacity(1500);
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        Result::<()>::Ok(())
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
}
