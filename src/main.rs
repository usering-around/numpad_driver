use numpad_driver::dev::NumberPad;
fn main() {
    let mut number_pad = NumberPad::new().unwrap();
    number_pad.enter_input_loop().unwrap();
}
