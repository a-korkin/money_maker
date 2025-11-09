```bash
sudo apt install -y libssl-dev libfontconfig1-dev cmake libx11-dev \
    libxrandr-dev xorg-dev libglu1-mesa-dev clang
cargo install sqlx-cli
```

```bash
cargo run -p app -- --secs=SBER --kind=candles --add
```
