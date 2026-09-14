fn main() {
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=APP_ENV");

    dotenvy::dotenv().ok();

    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
    let url_key = if app_env == "prod" {
        "SERVER_URL_PROD"
    } else {
        "SERVER_URL_DEV"
    };

    if let Ok(val) = std::env::var(url_key) {
        println!("cargo:rustc-env=SERVER_URL={}", val);
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    for var in ["WIFI_SSID", "WIFI_PASSWORD", "API_KEY"] {
        let val = std::env::var(var).unwrap_or_default();
        std::fs::write(out.join(var), val).unwrap();
    }

    embuild::espidf::sysenv::output();
}
