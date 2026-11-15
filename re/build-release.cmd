@echo off
cd /d "C:\Users\rrr\spotify-rs"
cargo build --workspace --release > "C:\Users\rrr\spotify-rs\re\out\release-build.log" 2>&1
echo DONE > "C:\Users\rrr\spotify-rs\re\out\release.done"
