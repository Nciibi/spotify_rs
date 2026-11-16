use std::fs;

#[test]
fn emit_scripts() {
    let dir = r"C:\Users\rrr\AppData\Local\Temp\opencode";
    fs::write(
        format!("{dir}\\sr-chrome.js"),
        spotify_host::chrome::injected_script(),
    )
    .unwrap();
    fs::write(
        format!("{dir}\\sr-adblock.js"),
        spotify_host::adblock::injected_script(),
    )
    .unwrap();
}
