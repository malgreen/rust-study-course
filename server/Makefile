.PHONY: pi

pi: 
	cross build --target aarch64-unknown-linux-musl
	scp target/aarch64-unknown-linux-musl/debug/server pi@co2pi.local:/home/pi/co2server
	ssh pi@co2pi.local "./co2server"
