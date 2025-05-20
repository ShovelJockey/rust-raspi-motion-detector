use std::{
    env::var,
    process::{Command, Stdio, Child},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;

pub fn test_initialise_camera() -> Result<std::process::Output, std::io::Error> {
    Command::new("rpicam-hello").arg("-t 100").output()
}

pub fn start_recording() -> u32 {
    let start = SystemTime::now();
    let time = start.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let save_path = var("VIDEO_SAVE_PATH").unwrap_or("/home".to_string());
    let output = format!("{save_path}/motion_{time:?}.mp4");
    println!("save arg: {output}");
    let rpicam_args = [
        "-t",
        "0",
        "--signal",
        "-codec",
        "libav",
        "--libav-format",
        "mpegts",
        "-o",
        "-",
    ];
    let camera_process = Command::new("rpicam-vid")
        .args(rpicam_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    let camera_process_id = camera_process.id();
    let ffmpeg_args = [
        "-f",
        "mpegts",
        "-i",
        "-",
        "-movflags",
        "faststart",
        "-f",
        "mp4",
        output.as_str(),
    ];
    Command::new("ffmpeg")
        .args(ffmpeg_args)
        .stdin(Stdio::from(camera_process.stdout.unwrap()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("FFMPEG video processing process completed successfully.");

    return camera_process_id;
}

pub fn start_stream_webrtc() -> Child {
    let mediamtx_dir = var("MEDIAMTX_PATH").unwrap_or("/home".to_string());
    let child_process = Command::new(format!("{mediamtx_dir}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    return child_process;
}

#[allow(dead_code)]
pub fn start_stream_hls() -> u32 {
    let command_args = [
        "-t",
        "0",
        "--inline",
        "--signal",
        "--listen",
        "-o",
        "tcp://localhost:8080",
    ];
    let child_process = Command::new("rpicam-vid")
        .args(command_args)
        .stderr(Stdio::piped())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    return child_process.id();
}

#[allow(dead_code)]
pub fn start_ffmpeg_hls_conversion(hls_dir: &TempDir) -> u32 {
    let output_dir = hls_dir.path().display().to_string();
    let child_process = Command::new("ffmpeg")
        .args([
            "-i",
            "tcp://0.0.0.0:8080?listen", // Input from RPi
            "-c:v",
            "copy", // No re-encoding (low CPU)
            "-f",
            "hls", // Output HLS
            "-hls_time",
            "4", // 2-second segments
            "-hls_list_size",
            "6", // Keep 5 segments in playlist
            "-hls_flags",
            "delete_segments", // Auto-delete old segments
            "-hls_segment_filename",
            &format!("{}/segment_%03d.ts", output_dir),
            &format!("{}/stream.m3u8", output_dir),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start FFmpeg");

    return child_process.id();
}

pub fn shutdown_cam_process(camera_process_id: u32) {
    Command::new("kill")
        .args(["-SIGUSR2", camera_process_id.to_string().as_str()])
        .output()
        .expect("SIGUSR signal sent to camera thread");
}

#[allow(dead_code)]
pub fn shutdown_ffmpeg_process(camera_process_id: u32) {
    Command::new("kill")
        .args(["-SIGUSR2", camera_process_id.to_string().as_str()])
        .output()
        .expect("SIGUSR signal sent to camera thread");
}
