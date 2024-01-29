use rppal::gpio::{IoPin, Gpio};
use rppal::gpio::Mode::Output;



pub struct SensorConfig {
    pub sensor_pin: IoPin
}

impl SensorConfig {

     pub fn new(gpio: Gpio, pin_num: u8) -> SensorConfig {
        let pin = gpio.get(pin_num).unwrap();
        let output_pin = pin.into_io(Output);

        return SensorConfig { sensor_pin: output_pin }
     }
}

pub struct  MotionDetector {
    pub sensor_config: SensorConfig
}


impl MotionDetector {

    pub fn new(pin_num: u8) -> MotionDetector {

        let gpio_interface = Gpio::new().unwrap();
        let sensor_config = SensorConfig::new(gpio_interface, pin_num);

        return MotionDetector{sensor_config};
    }

}
