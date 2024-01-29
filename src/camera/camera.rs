use std::process::Command;


pub fn initialise_camera() -> bool {
    let output = Command::new("sh").arg("rpicam-hello -t 100").output().expect("Expected camera hello to complete successfully");
    return output.status.success()
}

pub fn start_recording(current_time: String) -> String {
    let command_sring = format!("rpicam-vid -t 10000 -o {}.h264", current_time);
    let output = Command::new("sh").arg(command_sring).output().expect("Expected Camera command to succeed without error.");
    return output.status.to_string();
    }
