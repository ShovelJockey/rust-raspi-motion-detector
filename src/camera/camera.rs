use std::{process::{Command, Stdio}, time::{SystemTime, UNIX_EPOCH}};

pub fn test_initialise_camera() -> Result<std::process::Output, std::io::Error> {
    Command::new("rpicam-hello").arg("-t 100").output()
}

pub fn start_recording() -> u32 {
    let start = SystemTime::now();
    let time = start.duration_since(UNIX_EPOCH).unwrap();
    let output = format!("-o motion_{time:?}.mp4");
    println!("{output}");
    let command_args = ["-t 0", "--signal", output.as_str()];
    let child_process = Command::new("rpicam-vid")
        .args(command_args)
        .stderr(Stdio::piped())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    return child_process.id();
}

pub fn shutdown_process(camera_process_id: &u32) {
    Command::new("kill")
        .args(["-SIGUSR2", camera_process_id.to_string().as_str()])
        .output()
        .expect("SIGUSR signal sent to camera thread");
}

pub fn start_stream() -> u32 {
    let command_args = ["-t 0", "--inline", "--signal", "-o udp://localhost:8080"];
    let child_process = Command::new("rpicam-vid")
        .args(command_args)
        .stderr(Stdio::piped())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    return child_process.id();
}
