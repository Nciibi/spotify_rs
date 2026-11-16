use std::fs;
#[test]
fn emit() {
    let d = r"C:\Users\rrr\AppData\Local\Temp\opencode";
    fs::write(format!("{d}\\sr-chrome.js"), spotify_host::chrome::injected_script()).unwrap();
    fs::write(format!("{d}\\sr-adblock.js"), spotify_host::adblock::injected_script()).unwrap();
}
