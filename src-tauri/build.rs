fn main() {
    // 아이콘은 generate_context!로 바이너리에 들어가므로, 바꾸면 다시 빌드되게 한다.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
