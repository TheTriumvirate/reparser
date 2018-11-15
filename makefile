RUST_BACKTRACE=0
brain:
	cargo run --release -- gk2-rcc-mask.raw -o ./brain.bincode -w 148 -h 190 -d 160 -s 20 -T 0.1
	
helix:
	cargo run --release -- dt-helix.raw -o ./helix.bincode -w 38 -h 39 -d 40
