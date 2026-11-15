@echo off
call "C:\Users\rrr\spotify-rs\re\run-rea.cmd" analyze-javascript-application "C:/Users/rrr/spotify-rs/re/target/xpui" --json --analysis-timeout-ms 1800000 > "C:\Users\rrr\spotify-rs\re\out\xpui-analysis.json" 2>&1
echo DONE_XPUI > "C:\Users\rrr\spotify-rs\re\out\xpui.done"
