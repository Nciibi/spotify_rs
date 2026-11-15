@echo off
call "C:\Users\rrr\spotify-rs\re\run-rea.cmd" analyze "C:/Users/rrr/spotify-rs/re/target/Spotify.dll" --json > "C:\Users\rrr\spotify-rs\re\out\spotify-dll-analysis.json" 2>&1
echo DONE_DLL > "C:\Users\rrr\spotify-rs\re\out\spotify-dll.done"
