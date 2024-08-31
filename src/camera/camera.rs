use std::process::{Child, Command, Stdio};


pub fn test_initialise_camera() -> bool {
    let output = Command::new("rpicam-hello")
        .arg("-t 100")
        .output()
        .expect("Expected camera hello to complete successfully");
    return output.status.success();
}

pub fn initialise_camera() -> Child {
    // let command_args = [
    //     "-t 0",
    //     "--inline",
    //     "--initial 'pause'",
    //     "--signal",
    //     "-o udp://localhost:8080",
    // ];
    let output = Command::new("rpicam-vid")
        .arg("-t 100 --initial 'pause' --signal --listen -v 0 -o udp://localhost:8080")
        .stderr(Stdio::piped())
        .spawn()
        .expect("Expected Camera command to succeed without error.");
    return output;
}

pub fn start_stop_recording(camera_thread_id: &u32) {
    Command::new("kill")
        .args(["-SIGUSR1", camera_thread_id.to_string().as_str()])
        .output()
        .expect("SIGUSR signal sent to camera thread");
}
