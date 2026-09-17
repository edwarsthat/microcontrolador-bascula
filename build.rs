fn main() {
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SERVER_URL");

    dotenvy::dotenv().ok();

    let url =
        std::env::var("SERVER_URL").expect("Falta SERVER_URL: definela en .env o pasala al build");
    println!("cargo:rustc-env=SERVER_URL={url}");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    for var in ["WIFI_SSID", "WIFI_PASSWORD", "API_KEY"] {
        let val = std::env::var(var).unwrap_or_default();
        std::fs::write(out.join(var), val).unwrap();
    }

    embuild::espidf::sysenv::output();
}
