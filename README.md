Numpad driver for Asus ROG STRIX 2024 16 inch on linux. <br>

Build: Install rust, and then run: <br>
`cargo build --profile superopt` <br>
and use sudo to start the binary. <br>

Install as a service: <br>
`./install_service.sh` <br>
and then: <br>
`sudo systemctl enable --now numpad_driver.service` <br>

Note: currently there is a bug when using more than 1 finger. <br>

Todo: <br>
Fix the above mentioned bug <br>
Create a udev rule instead of requiring the binary to run as root
