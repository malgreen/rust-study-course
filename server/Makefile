.PHONY: pi influx

pi: 
	cross build --target aarch64-unknown-linux-musl --release
	scp target/aarch64-unknown-linux-musl/release/server pi@co2pi.local:/home/pi/co2server
	ssh pi@co2pi.local "./co2server"

influx:
	ssh pi@co2pi.local "sudo apt install docker.io docker-compose"
	scp compose.yml pi@co2pi.local:/home/pi/compose.yml
    scp influxdb.env pi@co2pi.local:/home/pi/influxdb.env
	

