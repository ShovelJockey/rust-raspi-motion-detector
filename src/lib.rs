pub mod camera;
pub mod email_alert;
pub mod motion_detect;


use std::time::Duration;
use std::thread::sleep;
use rppal::gpio::Mode::Output;
use rppal::gpio::{Gpio, IoPin};


fn main() {
    let gpio = Gpio::new().unwrap();
    let pin = gpio
            .get(4)
            .unwrap();
    let output_pin = pin.into_input();
    loop {
        if output_pin.is_high() {
            println!("you moved")
        } else {
            println!("no movement")
        }
        sleep(Duration::from_secs(1))
    }
    // if camera::camera::test_initialise_camera() {
    //     println!("camera check passed");
    //     detector_obj.monitor_loop()
    // } else {
    //     println!("Camera failed to initialise.");
    // }
}

// have timing being tracked on programme side.
// run recording indefinitely on start as command, then have 10 (5? or less maybe?) second time out being tracked in programme.
// if pin is detected high in this period, then continue recording for another timeout period
// Run pin check in own thread? want to have the check thread run until pin high then return true, otherwise return false?
// Use to extend or end recording with command - need to check if doing this is possible.
// could kill process? with streaming should be fine, might corrupt or fail if using saves
